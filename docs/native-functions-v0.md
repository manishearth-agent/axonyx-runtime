# Native Functions V0

This unreleased foundation registers server-side Rust callbacks explicitly. It
does not yet add `.ax` syntax, generated backend registration, preview parity,
HTTP endpoints, or activate rich-text storage in Legura.

```rust
use axonyx_runtime::native::AxNativeRegistry;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareContent { text: String }

let mut native = AxNativeRegistry::default();
native.register("content.prepare", |input: PrepareContent| {
    Ok::<_, ()>(PrepareContent { text: input.text.trim().to_owned() })
})?;
let output = native.call("content.prepare", serde_json::json!({"text": " hello "}))?;
# Ok::<(), axonyx_runtime::native::AxNativeError>(())
```

Registration happens during startup. Share the resulting immutable registry with
`Arc`; callbacks must be `Send + Sync`. Function names are bounded ASCII dotted
identifiers, not dynamic Rust symbols. Duplicate registrations fail without
replacing a callback. There is no process-global registry or dynamic library load.

One JSON value is deserialized into the declared input. Use
`#[serde(deny_unknown_fields)]` on request records to reject extra fields. The
result must serialize and deserialize as its declared Rust type. The default
budgets are 64 KiB encoded input and output, depth 32 (root depth 1), and 8192
value nodes including the root. Object keys contribute to encoded bytes, not
node count. Limits can be configured per registry. Public errors omit payloads,
serde details, callback errors and requested names.

This is **not** a zero-copy memory bridge, schema generator, or sandbox. Callers
already own the input `Value`; limits do not replace bounded HTTP parsing.
Callbacks are trusted Rust code and may allocate, block, panic or access captured
resources. Serialization may invoke custom user code. This API does not bound
CPU time or all application memory. Authentication, CSRF, authorization and
database transactions remain the action boundary's responsibility.

Next: specify explicit application registration and typed `.ax` invocation;
connect the same registry to preview and compiled execution; then integrate the
validated editor document without exposing arbitrary raw HTML.
