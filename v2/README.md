# Font Awesome — standalone Rust V2

Independent external executable for the `index` command (Search Font Awesome).
Schema 3, Rust process runtime, protocol 1, MSRV 1.85. No app, Tauri, local SDK
path dependency, JavaScript runtime, or installed profile is needed to build it.
This feature branch contains the standalone implementation and integration evidence.
Host installation and live acceptance remain pending.

Canonical identity: `com.tezbar.native.fontawesome`. Exact declared replacements:
`raycast.fontawesome` and `com.tezbar.fontawesome`. The V1 source remains intact
outside this directory apart from its short README discovery link.

- Source: `39549ced40cbe73492213e731064404dde84d165` from the authorized own repository.
- SDK 0.1.1: HTTPS git revision `e955911f0bf5b558f9d3eb5183df0adc95c22af2`.
- Host contract: `ea100cee3f69711de12485ba061950c5fc65258f` only.
- Raycast origin: `6734d85992290066fd31a7f5bee3cca3d3545a1a`.
- Original command PNG SHA256: `3cb5336951f849e1fea98816590ef96d7aaafa6be70a8c0ac92add03a7e1e063`.

`resources/command-icon.png` is byte-identical to that immutable Raycast PNG.
All 17 command style assets are copied without modification. Source, SDK and
host snapshots, immutable URLs and hashes are in [PROVENANCE.json](PROVENANCE.json).
The host's closed schema has no `origin` field; that metadata lives in this
provenance file rather than an invalid manifest extension.

The command searches Font Awesome 6.7.2 (48 results), offers 17 Pro family/style
choices and custom kits, and offers the original five ordered copy actions.
Without a custom API token it uses the original public free-token fallback and
forces the initial style to Classic Solid. Both free styles remain selectable.
A custom token requires `kits_read`, `domains_read`, and `svg_icons_pro`.
All five original preferences are supported. Kit filters match full names or
kit IDs, case-insensitively; kit search matches name or unicode locally.
Custom kit IDs are represented by opaque hashes in views and remembered state.
SVG copy writes literal text with the source's `<path fill="currentColor"`
replacement, not an SVG image clipboard.

Production HTTP is fixed to `https://api.fontawesome.com` and `/token`; TLS
validation is enabled, proxies and redirects disabled. Tests inject local HTTP
or locally trusted self-signed TLS only; production has no endpoint override.
Queries use JSON-escaped GraphQL string literals. Service errors never echo
bodies, URLs or credentials. OAuth/response caches stay in memory and are scoped
by credential, private data directory, and preferences. Only opaque remembered
selection IDs are persisted in `dataDir/fontawesome/storage.json` with atomic
replacement, corruption rejection, 32 KiB/64-scope bounds and symlink checks.

The process uses the SDK's serialized staged state plus an authoritative bounded
registry and revisioned, one-use action IDs. Discarded results cannot authorize
stale effects. A SDK-owned drop guard removes cancelled opens, disposed sessions
and EOF sessions. No detached port tasks, background lease, timer or polling is
used (`refreshMs: null`). Effects are consumed before awaiting a strict ACK.
An uncertain effect is never replayed automatically; its deliberate retry warns
that it may repeat the exact prepared effect. Request failures expose a reachable
Retry action, including from each item's action menu.

Build from this directory:

```sh
cargo build --locked --release
cargo test --locked
cargo +1.85.0 test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo +1.85.0 clippy --locked --all-targets -- -D warnings
python3 scripts/verify.py
```

`verify.py` prepares an ignored disposable V1 copy, installs its exact npm lock
with `--legacy-peer-deps`, invokes tests/typecheck through pnpm, executes the
original TS/TSX oracle, runs Rust and pinned host contract checks, builds the
native binary, collects license evidence, creates a private ZIP and regenerates
the source inventory. Node/pnpm, Python with venv, and installed Rust toolchains
are prerequisites; it does not install system compilers. A fresh HTTPS dependency
build is `python3 scripts/isolated-build.py --name isolation-new` (name must be
unused). `python3 scripts/package.py --binary <built-binary>` creates the
ignored deterministic ZIP and extracted package under `dist/`;
`python3 scripts/smoke.py` validates the extracted production executable without
live API calls or host effects. The ZIP is deterministic for identical binary and
asset bytes; different compiler paths/toolchains need not yield identical binaries.

[QA.md](QA.md) records actual validation and gaps. [INTEGRATION.md](INTEGRATION.md)
records host prerequisites, exact limits and compatibility adaptations.
[PACKAGE-INVENTORY.json](PACKAGE-INVENTORY.json) records native package digests
and every packaged file. [INDEX.sha256](INDEX.sha256) inventories the entire
Git-visible repository except itself. Binary ZIPs and build/QA outputs are ignored.
`artifacts` intentionally remains `{}`: no release URL, catalog pin or publication
has been invented. [ci.yml](ci.yml) is an inactive reference definition inside V2.
