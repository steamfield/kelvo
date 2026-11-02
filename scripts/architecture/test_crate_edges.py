"""Exercise the same jq policy invoked by make architecture-check."""
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parent


class CrateEdges(unittest.TestCase):
    def check_graph(self, packages):
        return subprocess.run(
            ["jq", "-e", "--slurpfile", "allowed", str(ROOT / "crate-edges.json"), "-f", str(ROOT / "crate-edges.jq")],
            input=json.dumps({"packages": packages}), text=True, capture_output=True,
        )

    def test_current_graph_and_external_dependencies(self):
        graph = json.loads((ROOT / "crate-edges.json").read_text())
        packages = [{"name": name, "dependencies": [
            {"name": dep, "kind": kind, "path": "/workspace/" + dep}
            for kind, deps in kinds.items() for dep in deps
        ] + [{"name": "serde", "kind": "normal", "path": None}]} for name, kinds in graph.items()]
        self.assertEqual(self.check_graph(packages).returncode, 0)

    def test_reverse_edge_fails(self):
        result = self.check_graph([{"name": "statglass-schema", "dependencies": [
            {"name": "statglass-engine", "kind": None, "path": "/engine"}
        ]}])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("forbidden normal edge: statglass-schema -> statglass-engine", result.stderr)

    def test_new_member_requires_review(self):
        self.assertNotEqual(self.check_graph([{"name": "statglass-new", "dependencies": []}]).returncode, 0)

    def test_test_only_edge_cannot_be_used_in_production(self):
        self.assertNotEqual(self.check_graph([{"name": "statglass-store", "dependencies": [
            {"name": "statglass-proto", "kind": None, "path": "/proto"}
        ]}]).returncode, 0)


if __name__ == "__main__":
    unittest.main()
