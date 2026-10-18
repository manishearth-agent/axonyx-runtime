# Private Binary Responses - 0.7.0 Candidate

Candidates: axonyx-core 0.7.0 and axonyx-runtime 0.7.0. This document records
release preparation, NOT crates.io publication. Macros and the internal state
WASM package are unchanged; no browser/WASM ABI change is implied.

## Compatibility Boundary

Core adds PrivateResponse to AxReturnPlan and InvalidPrivateResponse to
AxBackendCodegenError. Runtime adds InvalidBinaryResponse to
AxApiResponseValidationError. These public enum changes break exhaustive Rust
matches, so do not publish them under compatible 0.6.x dependency ranges.
The runtime candidate requires core 0.7.0. Existing published 0.6.x tooling must
continue to resolve only its old compatible core/runtime packages.

Consumers matching these enums must add explicit new cases. Binary is a reserved
HTTP response marker, not a data record or a replacement for JSON. Private routes
must use a final Native.privateResponse call in a GET route declared -> Binary.
The registered callback receives only a runtime-verified subject and small typed
JSON input; returned PNG/JPEG/binary bytes travel separately from Native.call.
Session verification, private/no-store headers, closed MIME and bounded bytes,
safe failure bodies and response validation are enforced. Preview explicitly
refuses this compiled-only route. See private-binary-routes-v0.md and
private-native-responses-v0.md for exact constraints and deferred work.

Empty native object/array payload generation (including nested combinations) is
also fixed. Existing JSON callback and API validation semantics remain unchanged.

## Evidence And Promotion Gates

Runtime PRs 264/265/266 passed both required gates. Framework PR 367 passed all
five gates and adopted CLI contracts plus an actual generated-production Axum
HTTP proof. Legura PR 64 passed all seven gates and proved private gallery images,
uploads, auth/CSRF, current-authority revocation, browser JS/no-JS rendering,
archive/recovery and independent session keys using exact source pins.

Those are source proofs, not registry-only verification of these candidates.
Prepare matching CLI/scaffolder/LSP dependency pins and app runtime defaults.
Require feature -> dev CI, then dev -> main release PRs and green main CI.
Publish core first, wait for index propagation, then package/verify/publish runtime.
Verify downstream tooling against registry 0.7.0 before publishing it. Do not
republish unchanged macros/UI/editor. Finally adopt registry packages in Legura,
rerun isolated full acceptance without source overrides, and create package-scoped
tags/releases only after actual publication and consumer proof.

No operator database, global CLI, deployed site, Docker image or product media
activation is changed by preparing this release. Per-response bounds are not a
sandbox or aggregate memory/DoS protection. Public images/attachments, thumbnails,
image repair UI, streaming and ranges remain separate work.
