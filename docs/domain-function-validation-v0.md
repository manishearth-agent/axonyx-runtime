# Domain function validation V0

Release candidate: core/runtime 0.6.8. Publication is pending release CI.

Pure Domain functions may return a declared record and use `.length` in their
calculations. Their generated Rust ABI now returns `Result<T, AxRuntimeError>`.
The author still declares `fn validatePost(...) -> PostValidation` and calls it
normally: Rust error propagation and ownership conversions are compiler details.

Declared calls are rewritten using token spans, including nested expressions,
validation conditions and JSON macros. String and other non-Copy arguments are
cloned; known record arguments are borrowed. Literal text, methods and qualified
external calls are not rewritten. Wrong argument counts produce a codegen
diagnostic instead of an invalid Rust call.

Record returns use checked serde conversion into the generated Rust record.
Mismatches return a generic runtime error, without exposing payloads or serde
details. This conversion is not zero-copy and does not imply full static return
expression checking. Record results stored as handler data retain JSON-member
rewriting, just like native callback data. `.length` preserves existing UTF-16
String semantics and list lengths, and errors on unsupported dynamic values.

Function parameter types are mapped through the existing backend type mapping
instead of emitting raw `String[]` or `Json` as invalid Rust types.

The existing native-consumer smoke now compiles and executes shared validation,
String reuse, nested helper calls, record construction/borrowing, String/list
lengths and rejected dynamic lengths/record payloads. Action valid/invalid
results are checked against preview execution. Run:

```text
cargo test -p axonyx-core
pwsh -File scripts/smoke-native-calls.ps1
```

An isolated Legura Posts HTTP run also exercises one imported validatePost from
create/update, the original field messages, exact/over-limit lengths,
authorization, rejected writes, slug conflicts, escaping and restart persistence.
It uses source overrides until this compiler change is released.

Optional/default helper parameters and database access inside pure fn remain
outside this change. Borrowed record calls still require actual typed records;
arbitrary JSON-to-record argument conversion is not added here.
