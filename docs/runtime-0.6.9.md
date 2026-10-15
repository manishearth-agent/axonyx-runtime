# Runtime 0.6.9

## Scope

This patch adds explicitly opted-in disabled textarea recovery after native
action validation failures. It allows rich editors to validate and restore an
unsaved document after full-page HTTP 422 navigation, rather than loading stale
stored content. See disabled-textarea-replay-v0.md for the exact HTML contract.

There are no new public AST variants, backend language changes, database
migrations or automatic editor activation. axonyx-core stays at 0.6.8; the CLI,
scaffold, LSP and UI do not need a new release for this behavior.

The server-rendered form must allowlist the field and mark its disabled textarea
with data-ax-retain-disabled="true". Replay preserves disabled state and uses
escaped text with data-ax-retained="true". Client code must validate this retained
value before enabling submission. No-JS rich editing is not enabled by replay.
The existing 64 KiB URL-encoded request budget, matching action/route checks,
duplicate/secret field exclusion and private form-result behavior remain intact.

## Release And Consumer Gates

The release candidate must pass runtime/all-feature tests, strict Clippy, native
calls, browser bridge and Postgres CI before promotion from dev to main. Package
verification must use the published unchanged core 0.6.8. Publication and tag
creation happen only after promotion; a candidate document is not proof that
crates.io already hosts this version.

After publication, applications should resolve runtime 0.6.9 and rebuild their
compiled server. Legura must update its lock files, then prove native 422 recovery
with registry dependencies and without RuntimeSource/EditorSource overrides.
The editor's separate Git pin and asset generation are not changed by publishing
the runtime. Its licensing/publication and explicit CMS activation remain separate.

Framework scaffolds currently use semver-compatible runtime 0.6.8 requirements;
fresh resolution can select the patch, while existing Cargo.lock files do not
update themselves. A checked dependency refresh/rebuild is required. No global
CLI reinstall or unrelated core publication is necessary for the runtime patch.
