# Verification evidence

Environment: Apple Silicon macOS, stable rustc 1.98.0 and installed Rust 1.85.0.
No user keys, paid/live API request, real clipboard, browser, Downloads folder,
installed app profile or live GUI was used. Tests use synthetic local HTTP/TLS,
SDK TestContext, mock React, disposable QA data and real OS pipes.

| Check | Actual result |
|---|---|
| Fresh HTTPS own clone, fetch, feature branch | Passed; source/HEAD `39549ced40cbe73492213e731064404dde84d165`, `feat/native-v2-rust` |
| XERJ first index/search | Generation 1 committed, 18/18 original code files indexed; three binary assets skipped, checked separately by hashes |
| XERJ final reindex | Incomplete: frozen dataset/schema rejected new v2/Cargo.toml docs family; required prefix retained, no restart/global changes or alternate-prefix bypass; exhaustive rg/Git/hash/full-file checks used |
| Stable Rust production/adapter/pipe suite | 10 tests passed; includes one guarded child-process entrypoint |
| Rust 1.85 full suite | Same 10 tests passed |
| `cargo fmt --check` | Passed |
| Strict all-target Clippy stable and 1.85 | Passed |
| Executed original TS/TSX oracle | 720 copy cases, 17 query/style cases; original preferences, access-token, kit/data hooks, command, actions and filter functions executed with injected producers |
| Original V1 pnpm smoke/typecheck | Passed in ignored disposable copy; root package lock unchanged |
| Original V1 utility tests | 10 passed through pnpm/tsx |
| Actual pinned schema | Source manifest valid; six negative mutations rejected |
| Actual pinned Grid renderer | Adapter view, eight columns, controlled style/kit, primary action, reachable Retry, inert data image and empty error passed with mock React |
| Separate host patch regression harness | Two tests passed against extracted actual patched functions; full app build not run |
| Native ARM64 release | Built; Mach-O arm64 verified |
| Fresh Cargo home isolated source build | Passed, HTTPS SDK git pin, no local path/app/host dependency; final run `qa/isolation-verified` |
| Deterministic private ZIP | Two identical passes from the same isolated native binary; inventory and digests in PACKAGE-INVENTORY.json |
| Extracted production process | Passed hash inventory, protocol-info/self-test, hello/init, invalid-credential pre-network validation, shutdown, EOF and malformed-frame negatives |
| Windows GNU check | Attempted; compiler-blocked in ring because `x86_64-w64-mingw32-gcc` is absent; no system toolchain installed |
| Windows MSVC | Not run |
| Real macOS/Windows app GUI, install, preferences UX, clipboard | Not run; manifest verification remains not-run for both platforms |
| Live Font Awesome public/Pro service, real key/account/kits | Not run |
| Publication, push, PR, release, clone deletion | Not performed |

The HTTP/TLS suite exercises credential/free/Pro/profile/settings cache separation,
cache hit/expiry, simultaneous scoped requests, permission failures and invalid
tokens, self-signed TLS rejection and explicit local trust, redirect refusal,
429, malformed JSON, oversized/over-row responses, reflected sensitive data and
GraphQL partial-error rejection. It performs no public-network request.

The SDK OS-pipe suite exercises initialize/open/search, superseded out-of-order
queries and A/B/A, stale action rejection, failed-query Retry, strict contradictory
ACK, deliberate exact-effect retry, cancellation of search/open/reverse effects,
late ACK without replay, revision reconciliation, refresh, dispose, late action,
EOF cleanup and process exit. Adapter tests cover style/kit selection, primary
action order, remembered scope isolation, corrupted/oversized/symlinked storage,
invalid preferences, safe SVGs and discarded staged responses. No assertion
claims that clipboard/network/storage effects roll back when a request is dropped.

`npm ci` with default peer resolution initially rejected the original lock's
React-type peer choices. `npm ci --legacy-peer-deps --ignore-scripts` successfully
installed the **original exact lock** in QA; pnpm scripts were then run with
`verify-deps-before-run=false` to avoid re-resolving it. The initial auto-install
attempt's resolved versions were not used for final locked-source evidence.
An early isolated build emitted an optional rust-objcopy/libLLVM stripping warning
but completed; the final fresh-home build explicitly disabled stripping and
completed without that warning. No global tool configuration was changed.

The full reproduction script completed successfully; after the final decoded-credential
reflection check, both Rust suites, both strict Clippy runs, renderer/schema checks,
fresh-home build and extracted production smoke were rerun.

The source oracle executes pinned source in a VM with mocked hooks/effect
producers; undeclared modules, network, filesystem, clipboard and browser effects
are blocked. React hooks are injected rather than a real Raycast process. The
pinned renderer probe similarly executes component code with mocked React, not a
browser or app UI. The production executable smoke intentionally uses an invalid
preference to prove pre-network rejection; successful remote-service interaction
is covered only with the same adapter under synthetic local producers. These
are compatibility evidence, not live service, installed-host or universal parity.

License evidence: original own and Raycast MIT, SDK Apache-2.0/NOTICE and available
locked crate texts are included. `DEPENDENCIES.json` explicitly records missing
top-level text for r-efi 6.0.0 and yasna 0.5.2; their declared licenses are metadata,
not invented grants. `SOURCE-DEPENDENCIES.json` records the original npm lock and
available installed QA package texts; optional/noninstalled evidence gaps are
explicit. JavaScript dependencies and all test fixture credentials are excluded
from the executable ZIP. The source's embedded **public free fallback** is compiled
for the authorized no-token flow; custom credentials are never packaged or logged.

Reproduction: `python3 scripts/verify.py`. Fresh-home run uses
`python3 scripts/isolated-build.py --name <unused-name>`. Final artifact was
packaged with `python3 scripts/package.py --binary
qa/isolation-verified/target/release/fontawesome-v2`, then `python3 scripts/smoke.py`.
Ignored QA logs remain in `qa/` for parent inspection. Nothing is staged or
committed; root status must remain README modified plus V2 untracked.
