# Native Functions V0

This unreleased foundation registers server-side Rust callbacks explicitly. It
supports server-side `.ax` data bindings through the same registry in preview
and generated Rust. It does not yet add automatic CLI/application registration,
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

## Backend Data Bindings

```ax
route POST "/prepare" {
  data prepared = Native.call("content.prepare", {text: request.form.text})
  return json(prepared)
}
```

The name must be a literal source-owned registered name, not a request field.
There is exactly one payload. Calls are supported as backend data bindings,
not inside pure domain functions. Bind computed values before passing them to
the callback. The Rust input/output are checked by the registry; this initial
API does not infer an ASX record type or generate a native function schema.

Local native results support JSON member access, for example `prepared.columns`,
`prepared.items[0].text`, and `(prepared.columns).body`. Simple local aliases of
these values retain JSON member access. The compiler lowers these members to JSON
indexing, not Rust struct fields; unrelated typed record fields and string literals
are unchanged. A missing member (including a member of null) produces null. This
does not infer a typed native result or make arbitrary method calls JSON aliases.

Configure the runtime at startup with `runtime.with_native_functions(native)`.
The existing `AxRuntimeEnvAccess` capability interface (which already carries
sessions) has an optional `runtime_native_call` method, defaulting to unavailable
so existing custom runtimes remain source-compatible. `AxDatabaseRuntime`
delegates it to its configured registry. Preview must receive this same
configured runtime; there is no mock success or fallback to an empty value.

`pwsh -File scripts/smoke-native-calls.ps1` builds and runs a separate generated
Rust consumer and compares it with preview. It covers valid input, nested
objects/lists, form/query values, invalid types, missing names, callback failure,
payload limits, member/alias access, action guards and absent registration. It uses a new temporary application
and an in-memory runtime, not an operator database.

Next: explicit application/CLI package registration shared by development and
production; field-aware recovery and typed rich output; then integrate the
validated editor document without exposing arbitrary raw HTML. No package
release or live CMS integration is implied by this unreleased foundation.
