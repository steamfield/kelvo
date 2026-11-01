---
paths:
  - "crates/**/*.rs"
  - "crates/**/Cargo.toml"
  - "src-tauri/**/*.rs"
  - "src-tauri/Cargo.toml"
  - "Cargo.toml"
---

# Rust Crate Rules

Architecture lives in `plan/architecture.md`.
Load the `ce:architecting-systems` skill before adding a crate, moving code between crates
or changing a dependency edge.

## Crate boundaries

The executable dependency policy is `scripts/architecture/crate-edges.json`.
`make architecture-check` compares normal, build and dev edges against Cargo metadata,
including inactive target-specific dependencies. Cargo alone prevents cycles; this check
also prevents an otherwise valid dependency from crossing an ownership boundary.

Current normal dependencies (left side depends on the right):

```
kelvo-schema        -> none
kelvo-proto         -> schema
kelvo-collect       -> schema
kelvo-store         -> schema
kelvo-engine        -> schema, collect, store
kelvo-remote        -> schema, proto, store, engine
kelvo-remote-helper -> remote
kelvo-agent         -> schema, proto, store, engine, remote
kelvo (src-tauri)   -> schema, proto, store, engine, remote
```

The policy records dev edges separately (including store's protocol fixtures). Do not
copy a new dependency into the policy simply to silence a failure: explain the ownership
change in the implementation/decision record and test the boundary. Authorized refactors
can proceed autonomously within their agreed scope.

- `kelvo-schema` is pure data: metric catalog, series keys (`metric_id` + labels), HostInfo,
  Capabilities, alert-rule data, the typed `Snapshot` view. No I/O, no platform code.
  The one non-data module is `kelvo_schema::lock` (`LockExt`/`RwLockExt`, poison-recovering
  lock access, std only): every crate already depends on schema, so it lives here rather
  than adding a crate edge.
- `kelvo-proto` is framing, codec (CBOR via `ciborium`), handshake and message types. It
  never touches the store or collectors.
- `kelvo-collect` reads the OS. It knows nothing about storage or the UI.
- `kelvo-store` knows SQLite and series layouts. It knows nothing about collectors.
- `kelvo-engine` wires collectors to the store and bus through the `Ticker` and
  `PowerSignals` traits. It also owns `HistoryStore` lifecycle, generation-safe reader
  checkout/reset and persisted/recent history query merging.
- `kelvo-remote` owns SSH/session orchestration and `Mirror` storage synchronization;
  `kelvo-remote-helper` is the bundled stdio-to-local-socket bridge, not a collector agent.
- `kelvo-agent` is the headless collector for hosts without the app (D-116): the engine,
  its own history and the metrics socket, plus the one-shot commands the controller runs
  over SSH. Install paths and unit text live in `kelvo_remote::agent_layout`, shared
  with the controller.
- `src-tauri` is a thin shell: commands, windows, tray, settings ownership. Business logic
  that isn't Tauri-specific belongs in a crate, where it is testable without a webview.

Zero runtime dependencies (D-058): the app links only `/System/Library/` frameworks and
`/usr/lib/` libraries. A crate with a C library must use its bundled or vendored build
(like `rusqlite`'s `bundled`); never require a Homebrew or system install. Dev-only tools
(macmon for the accuracy script) stay optional. `make check-deps` enforces this on
the built binary locally before release.

## Data model rules

Series storage and explicit gaps are in CLAUDE.md and `data-boundary.md`. On top of those:

- The typed `Snapshot` is a view built from series and is never stored or synced.
- `host_id` (a persisted UUID) threads through every store call, command and channel.
- Exactly one process writes a given SQLite file.
- `seq` and millisecond timestamps are `i64` and stay below 2^53 (documented in
  `kelvo-schema`) so they cross to TS as `number`.
