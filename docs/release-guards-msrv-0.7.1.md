# Request Guards And MSRV - 0.7.1 Candidate

Candidates: axonyx-core 0.7.1 and axonyx-runtime 0.7.1. Preparation is not
publication confirmation. Macros, UI/editor and the internal state WASM package
are unchanged; no new public enum cases or browser/WASM ABI are introduced.

Core resolves query/params/request members within compound backend expressions
without changing literal text or same-named record/module members. Compound
Auth.subject expressions receive the server-owned session context; requestless
Domain helpers cannot implicitly read it. Membership permits fallible request
operands outside boolean closures; AND/OR retain short-circuit behavior.
See compound-request-expressions-v0.md for the exact tested boundary.

Runtime bounds Tokio to >=1.52.3, <1.54: Tokio 1.54 requires Rust 1.85 while Axonyx
declares Rust 1.82. The bound remains active in default consumers because
PostgreSQL already requires Tokio. The existing tokio feature is preserved.
The fresh-lockfile default/Tokio/Axum consumer check prevents a cached lockfile
from hiding dependency drift. Missing query values retain their existing error
contract; query fields are not authentication.

Source PRs 269/270 and framework adoption PR 371 passed mandatory CI and merged
to dev. Candidate PRs must pass their gates before dev adoption, followed by a
separate dev-to-main release PR and green main CI. Build-verify/package/publish
core first, then runtime against published core 0.7.1. Do not bypass verification
because the upstream crate is not yet visible in the index.

After matching CLI/scaffolder/LSP publication, verify fresh isolated registry
installs and consumers, then create package-scoped Git tags/releases. Operator
CMS, DB/configuration, global tools and deployed sites are not automatically
updated. These fixes do not turn Axonyx into a general TypeScript implementation
or constitute a production security audit.
