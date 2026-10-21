# Public Native Binary Responses - 0.8.0 Candidate

Preparation only, not publication confirmation. Candidates: axonyx-core 0.8.0 and
axonyx-runtime 0.8.0. Macros, internal state WASM, UI/editor and browser/WASM ABI
are unchanged. Framework CLI, scaffolder and LSP must adopt matching 0.8.0 pins.

## Why A Minor Boundary

AxReturnPlan::PublicResponse is a new public enum variant. Exhaustive downstream
matches break, so do not backport it into a compatible 0.7.x release or allow an
old published CLI to resolve this core via its existing dependency range.

Runtime PR 273 introduced explicit public binary registration and GET-only
anonymous admission. Compiler PR 274 added final Native.publicResponse returns
in GET -> Binary routes and rejected illegal/nested binary-as-value calls. Both
passed required exact-candidate CI before dev merge. Framework PR 378 adopted
all CLI matches/discovery/contracts and proved actual compiled HTTP after its
five required checks passed. Conservative private/no-store headers and product
authorization remain mandatory; public admission does not grant arbitrary media
access or permit dispatch to a private callback.

Native application callbacks remain compiled-only. Interpretive preview refuses
registration explicitly, rather than silently emulating application Rust. No
Legura public endpoint, browser proof, CDN caching or production security audit
is claimed by this release candidate. Independent grouped-coalescing guard
parser issue (framework 377) remains open; it is not silently included as fixed.

## Promotion Gates

1. Version candidate PRs target dev and pass all required checks on exact heads.
2. Separate dev-to-main release PRs, then green exact main CI before publication.
3. Build-verify/package/publish core first; wait for registry propagation.
4. Build-verify runtime against actual published core 0.8.0, then publish runtime.
5. Verify/package/publish matching CLI/scaffolder/LSP against registry 0.8.0.
6. Fresh registry-only tools/consumers: Rust 1.82, templates, generated compiled
   public/private binary HTTP, grant revocation, safe errors and private logout.
7. Only then create package-scoped tags/releases and record actual evidence.

Do not bypass package verification or claim --list rehearsal proves uploaded
dependencies compile. Do not silently change global tools, the operator CMS,
its .env/database/process, deployed sites or Docker images. Legura adoption and
real public-image markup/fallback acceptance are separate bounded follow-ups.
