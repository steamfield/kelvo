# Rename to StatGlass

Approved 2026-11-01. Implementation uses GPT-6.1 Terra subagents with the
`ce:executing-plans` workflow. A Terra devil's advocate reviewed the plan against
the source before implementation.

## Agreed behavior

The display name is StatGlass. Lowercase identifiers use `statglass`; uppercase
environment variables use `STATGLASS`. Rename filenames, crate directories,
package names, imports, commands, brand assets and active documentation.

StatGlass starts fresh by default: new bundle identifiers under `com.tryopendata`, app and
agent data/log paths, sockets, service names and identity-binding prefixes.
Existing installations and their data remain untouched. There is no migration or
legacy alias. Preserve storage schemas, metric IDs, CBOR versions and identity
algorithms. Serialize the renamed self-process refusal as `statglass` explicitly.

The user subsequently requested a one-off copy of this Mac's existing release
and development data. Both were copied to their matching StatGlass directories
using SQLite backups that include committed WAL data. The UUIDs were retained
and their machine bindings verified and regenerated for the new namespace.
All seven database snapshots passed integrity, foreign-key, row-count and
coverage checks. Original data remains in place. Copied remote connection paths
use StatGlass names; those remote installations still need separate upgrades.
This local operation does not add automatic migration to the app.

Keep the heat-grid icon. Rebuild the two outlined SVG wordmarks as StatGlass in
SF Pro Semibold, retaining grid geometry, colors and baseline.

Rename GitHub to `steamfield/statglass` after local verification. New releases
use `statglass-latest.json`, `statglass-release.json` and
`statglass-release.json.sig`, with StatGlass app archives and lowercase agent
names. The new catalog accepts only canonical `steamfield/statglass` artifacts.
Do not publish, bump versions, alter historical release assets, or push commits.

Preserve historical evidence, frozen source provenance and production-signed
test payload bytes. Preserve production signing keys, including the existing
local backup path `~/.config/kelvo/release/updater.key`; the rename does not move
that secret. Re-sign the catalog test fixture with a disposable test key.

Generated TypeScript and shared agent guidance are regenerated from sources.
Lockfile changes must preserve dependency versions. The attached checkout's root
directory is not renamed. Preserve all unrelated work, and recheck the primary
checkout before local integration; never stash or reset other changes.

## Work items

- [x] Review plan against source and create an isolated worktree.
- [x] Implement and apply the tracked-file rename script (dry run, apply, check).
- [x] Verify script safety with temporary Git fixture tests.
- [x] Complete release-channel, serialization, signature-fixture and generated-file work.
- [x] Rebuild wordmarks and verify the longer name in browser screenshots.
- [x] Run project checks and packaged-app checks; user accepted QA as complete.
- [x] Complete independent Terra code review and fix findings.
- [x] Copy the user's local release/dev data and adapt namespace bindings on request.
- [x] Rename GitHub and verify current media and historical release links.
- [x] Commit and merge locally (`98e80ed`).
- [ ] Archive the worktree: Codex retained it as this chat's protected primary checkout.

## Verification

Use scoped checks while iterating, then the documented full checks: frontend and
Rust type checks, `make check`, `make bindings-check`, architecture and agent-config
fixtures, functional browser tests and `make e2e-perf`. Hosted workflows stay on
Linux. Native packaging must not install over Kelvo. Check authentic signatures,
tampering rejection, canonical catalog URLs, agent paths and isolated fresh startup.

Inspect onboarding, sidebar, popover, settings, the dev gallery and both wordmarks.
Record evidence and any differences from the design system. Finish with no
unexplained legacy-name matches and no generated-file drift.

The 16 disposable-repository script tests pass. They reproduce and constrain
tracked destination collisions, symlink traversal and destination parent files
before writes. Release manifest tests (3), updater signature/catalog tests (6),
and the scoped identity suite (15 passed, 1 hardware-only test ignored) pass. The
identity expectations were recalculated independently for the D-125 namespace.

Terra code review approved the final implementation after fixes to destination
preflight, Bun's root workspace metadata and a stale README download instruction.
It independently checked the new identity test hashes. Ten browser captures in
each before/after set cover onboarding, dashboard, popover, settings and the dev
gallery in both themes. The name fits without horizontal overflow or clipping;
page-script errors were absent. The visible differences are the new name, wider
outlined wordmarks (152.60 by 32), and the process-name initial, as required by the
rename. Icon geometry, type scale, surfaces and layout stay the same.

Frontend checks passed 1,384 tests; functional Chromium/WebKit passed all 500 cases.
The updater dependency's 26 tests pass. Native app and DMG packaging, deep signature
verification, system-library checks and both Linux ELF alignment checks pass.
The verified app in `/Applications/StatGlass.app` launched with live readings and
the copied historical timeline. Its host UUID is unchanged and startup logs have
no errors. Launch at Login moved to StatGlass; Kelvo's setting is off. The original
app and data remain available.

The full quality run stopped at the real-process allocation budget. Alternating
unchanged-main and renamed binaries both passed and failed that same gate, with
different process-turnover counts; the collector and harness have no logic changes.
No budget was raised. The user inspected the running app and explicitly accepted
QA as complete, so further performance and freshness reruns were stopped. Prior
failures and diagnostic evidence remain in `target/qa/rename-statglass/` in the
primary checkout.

GitHub is now `steamfield/statglass`; its media URL and the historical 0.3.2 release
link both return HTTP 200. No release, tag or source push was performed.
