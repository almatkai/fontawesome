# Integration and compatibility

This port targets host `ea100cee3f69711de12485ba061950c5fc65258f`. Only immutable
`git show` output was inspected in the concurrent dirty app checkout. No app
working files, branch, build, profile, SDK or sibling extension were changed.

The schema requires `com.tezbar.native.fontawesome`; source own ID
`com.tezbar.fontawesome` is an exact replacement alongside `raycast.fontawesome`.
The only command is `index`, view mode, serialized handlers, 60-second host
budget, and no polling. Manifest source artifacts are empty. An actual install
still needs parent-reviewed per-target artifact metadata and a trusted local
release reference or published release. Nothing was installed or published.

## Separate host prerequisites — unapplied

`integration/0001-active-resources-exact-replacements.patch` is generated against
the pinned host snapshots. It proposes two narrowly required generic fixes:

1. Keep canonical installation/preference root separate from the resolved active
   version's resource root. Resolve command and extension icons in that resource
   root so the exact packaged Raycast PNG can be discovered.
2. Resolve own/Raycast replacements by exact declarations in verified active
   manifests, only for covered Rust commands on compatible platforms. Reject
   ambiguous aliases; do not infer an own replacement by stripping a prefix.
   Wire that verified selection into registry/native command routing.

The patch includes two regression tests. Extracted actual patched functions were
compiled and their tests run independently. Full app compilation, installation,
registry discovery and routing in a live app have **not** been run. The patch is
not applied to any app checkout and requires parent review. The host's existing
catalog is not changed. No unused Detail primary callback, List dropdown, tooltip,
shortcut or other renderer patches are included: this command uses Grid.

## Renderer adaptation

The actual pinned `grid-runtime.tsx`, `grid-content.ts` and `extensionImages.ts`
were executed with mock React hooks and the real adapter-generated view. It
accepts Grid, Grid.Item, Grid.EmptyView and a `List.Dropdown`-shaped accessory,
with controlled `value`, `actionId` and `{formValues:{value}}`. Row copy actions
are ordered exactly as the original; utility Retry/Preferences actions follow
them to remain reachable. The selector has no redundant action-menu entry.
Eight columns are supported (the renderer clamps to 2..8), server filtering is
enabled, and no arbitrary HTML is emitted. Protocol flags/manifest identity
and reverse clipboard `{text}`/settings `{surface}` shapes were verified.
No shell or clipboard-read capability is requested.

Preview images contain only bounded reconstructed SVG paths in an inert data
image; API SVG is never active HTML. DTDs/entities, arbitrary child elements,
URLs, styles and handlers do not pass through. The sanitizer retains path data
and numeric opacity, rebuilding dimensions and a fixed neutral fill. Unsupported
SVGs fall back to the icon label/initial rather than forwarding arbitrary content.
This intentionally adapts Raycast's theme tint and SVG subset: theme color,
per-item tooltips, dropdown section/icon decoration and exact Raycast spacing
are not reproduced by this renderer. Arbitrary user/image-host URLs never enter
views. Source SVG copying remains literal text and is not sanitized into a
possibly different SVG; the host receives **only** `{text}`, never HTML.

Grid EmptyView text and global Actions expose Retry/Preferences. Grid does not
render the source's nested empty-view ActionPanel directly; the action descriptors
are exposed in the renderer's Actions menu. Successful copies emit the original
HUD message after strict `{ok:true,message?:string}` ACK. Null, booleans,
`ok:false`, unknown fields, non-string message, and contradictory `error` fields
are rejected. Late/rejected/timed-out ACKs leave a warning and an exact explicit
prepared-effect retry. No extra source effect confirmation is added.

## Intentional data/runtime differences

- The original query interpolates raw search text. V2 uses equivalent JSON-escaped
  GraphQL literals and JSON request bodies, fixing quote/newline injection.
- OAuth is volatile and credential/profile/settings scoped; no credentials or
  API responses are written to disk. Expiry is measured from acquisition start,
  limited to the lesser of API `expires_in` and the original one hour. Pro scopes
  are checked before kit/search requests. Cache entries expire after five minutes;
  at most eight credential/settings scopes each retain one result and kit list.
  Failed requests clear cached authorization and data; deliberate Retry reacquires.
  The original Raycast cache persists across process restarts and can display
  stale prior query/type results. V2 deliberately withholds stale results during
  failed or discarded operations and never shares them across scopes.
- Remembered selection is also settings scoped. Changing credentials, private
  profile, default style, primary action or kit filter starts a different scope.
  The original uses a global `lastType`; no legacy profile cache is imported.
- Kit tokens are opaque hashed selector IDs. Source `findKitByToken`/first-kit
  fallback is retained in normalization, while the adapter rejects unavailable
  selections. Kit unicode numbers normalize to text; malformed kit geometry or
  shapes fail the complete request. Source permissive inputs are not reproduced.
- `String.fromCharCode(parseInt(hex,16))` 16-bit wrap and number rounding are
  preserved, including non-BMP wrap. Unpaired UTF-16 surrogate code units cannot
  be represented by Rust/JSON strings and become U+FFFD. No full Unicode-scalar
  correction is silently substituted for the original behavior.
- Source hooks' asynchronous loading/keep-previous-data display and toast timing
  are adapted to bounded request/response views. Ordinary request failures gain
  deliberate Retry. Concurrent source kit/search partial-success behavior is
  replaced by complete validated results or a visible error, never partial success.
  Host preferences are immutable per SDK session; save settings then reopen.

## Bounds and lifecycle

Search text is limited to 1024 UTF-8 bytes. Two HTTP requests can run concurrently,
with three-second connect, eighteen-second request and fifty-second operation
bounds. Redirects are rejected. JSON is limited to 2 MiB/40,000 nodes; standard
search to 48 rows, kits to 128, uploads to 512 per kit, SVG to 64 KiB and previews
to 256 XML nodes. Labels, unicode, path data and dimensions are validated. Limits
fail the whole response; result truncation is not presented as successful parity.
The SDK additionally bounds frames, sessions, task scopes and reverse calls.

The adapter registry bounds 128 sessions. Every emitted intent is tied to a
fresh authoritative revision and consumed before awaits. Staged SDK state only
commits on successful current operations. On reconciliation a discarded revision
cannot restore rows or authorize a clipboard action. Cancellation may leave
external network/storage/effects completed: they are never described as rolled
back. An SDK-owned pending cleanup future carries a drop guard so cancelled
session opens are removed even if dispose is omitted. No independently detached
threads/tasks, refresh timers or polls are added by the port. SDK-owned stdin
pump/runtime internals remain governed by its pinned lifecycle contract.
