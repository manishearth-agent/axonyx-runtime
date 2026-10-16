# Readable ASX Formatting V1 Candidate

- axonyx-core 0.6.9
- axonyx-runtime 0.6.10 (updated core dependency)

The compiler-owned formatter expands complete element-only ASX containers and
wraps long opening tags with one attribute per line. It preserves mixed prose,
raw content and embedded JS/style source. Parsed markup equality gates expansion
and wrapping. Balanced mixed inline subtrees do not leak prose quotes, URLs or
tags after text into structural indentation; regression checks assert repeated
formatting stability.

Legacy indentation-sensitive declarations and query clauses remain untouched.
This is not a syntax migration, a new WASM ABI, or a CMS/editor upgrade. Runtime
server behavior and the existing bridge remain unchanged by this release.
axonyx-macros and axonyx-state-wasm are not republished.

The formatter reaches users through the matching CLI and LSP packages. Updating
only an application's runtime dependency does not replace an installed CLI/LSP.

Release gate: green dev candidate, dev -> main PR, green main, then publish core
before runtime. Wait for crates.io propagation before packaging/publishing runtime
and downstream tooling. Package preparation against an unpublished core can fail
before propagation; that is not evidence that the runtime was published.
Create package tags and GitHub releases only after registry and consumer proof.

This document records a candidate, not registry publication. Existing apps,
operator databases, editor asset generations and production sites are not
automatically upgraded.
