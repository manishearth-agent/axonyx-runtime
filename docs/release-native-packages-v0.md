# Native Packages V0 Release Candidate

axonyx-core 0.6.7 and axonyx-runtime 0.6.7.
Native calls are explicit names resolved through a typed Rust registry.
Input/result contracts and missing/error cases fail closed. Native JSON member
lowering preserves literal text, typed record fields and nested/indexed access.
Compiled/preview contract parity and an actual compiled consumer are covered.

This is not arbitrary Rust invocation, browser shared memory or a released CMS.
Application startup wiring ships separately in cargo-axonyx 0.6.11.
No changes require republishing macros or state-wasm.

After required CI and merge to main, publish core before runtime and wait for
crates.io propagation. Verify packaged builds and a registry-only app consumer
before marking publication complete or creating release tags.
