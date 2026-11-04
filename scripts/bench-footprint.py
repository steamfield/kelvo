#!/usr/bin/env python3
"""Footprint checks for the coalition benchmark (plan/rfc-memory-footprint.md, phase 1).

Usage:
  bench-footprint.py check SCENARIO_JSON LOG_DIR LAUNCH_EPOCH END_EPOCH HAS_ACTIONS
      Fail a run whose numbers are not this launch's: a coalition member that started
      before the app (its lifetime peak predates the launch), or, when the scenario ran
      STATGLASS_BENCH_ACTIONS, a step that changed nothing or a list that did not finish
      inside the measured window.
  bench-footprint.py report BUDGET_JSON OUT_DIR SCENARIO...
      Print each scenario's median and peak against perf-budget.json's footprint
      section. Exits 1 only when a scenario is over its ceiling and footprint.blocking
      is true.
"""

import glob
import json
import re
import sys
from datetime import datetime
from pathlib import Path

LINE = re.compile(r"^(\S+Z)\s+(\w+)\s+statglass_lib::bench(?:::actions)?: (.*)$")


def check(scenario: Path, logs: Path, launch: float, end: float, has_actions: bool) -> None:
    data = json.loads(scenario.read_text())
    if data.get("stale"):
        raise ValueError(
            f"{', '.join(data['stale'])} started before this launch; its peak is not this run's"
        )
    if not has_actions:
        return
    done = False
    problems = []
    for path in sorted(glob.glob(str(logs / "statglass.*.log"))):
        with open(path, errors="replace") as lines:
            for line in lines:
                match = LINE.match(line)
                if not match:
                    continue
                try:
                    stamp = datetime.fromisoformat(match[1].replace("Z", "+00:00")).timestamp()
                except ValueError:
                    continue
                if not launch <= stamp <= end:
                    continue
                if match[3] == "bench actions done":
                    done = True
                elif match[2] == "WARN" and "bench action" in match[3]:
                    problems.append(match[3])
    if problems:
        raise ValueError("bench actions failed: " + "; ".join(problems))
    if not done:
        raise ValueError("bench actions did not finish inside the measured window")


def report(budget_path: Path, out: Path, scenarios: list[str]) -> int:
    budget = json.loads(budget_path.read_text()).get("footprint", {})
    baselines = budget.get("baseline", {})
    regress = budget.get("regressionPct", 20)
    blocking = budget.get("blocking", False)
    target = budget.get("target", {}).get("trayOnlyMb")
    failed = False
    for name in scenarios:
        data = json.loads((out / f"{name}.json").read_text())
        median, peak = data["footprint_median_mb"], data["footprint_peak_mb"]
        if name == "tray" and target is not None:
            side = "over" if median > target else "under"
            print(f"bench: tray-only footprint {median:.1f} MB is {side} the {target} MB target")
        base = baselines.get(name)
        if not base:
            print(f"bench: {name} footprint {median:.1f} MB median, {peak:.1f} MB peak (no baseline)")
            continue
        for label, value, key in (("median", median, "medianMb"), ("peak", peak, "peakMb")):
            ceiling = base[key] * (1 + regress / 100)
            if value > ceiling:
                note = "" if blocking else ", advisory"
                print(
                    f"bench: {name} footprint {label} {value:.1f} MB is over its ceiling "
                    f"{ceiling:.1f} MB (baseline {base[key]} + {regress}%{note})"
                )
                failed = failed or blocking
            else:
                print(f"bench: {name} footprint {label} {value:.1f} MB is under {ceiling:.1f} MB")
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    if len(argv) == 7 and argv[1] == "check":
        try:
            check(Path(argv[2]), Path(argv[3]), float(argv[4]), float(argv[5]), argv[6] == "1")
        except ValueError as e:
            print(f"bench-footprint: {e}", file=sys.stderr)
            return 2
        return 0
    if len(argv) >= 4 and argv[1] == "report":
        return report(Path(argv[2]), Path(argv[3]), argv[4:])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
