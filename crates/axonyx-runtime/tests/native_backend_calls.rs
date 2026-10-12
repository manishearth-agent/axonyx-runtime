use axonyx_core::ax_backend_codegen::compile_backend_ax_to_module;
use axonyx_runtime::backend::{AxDatabaseRuntime, AxEnv, AxRuntimeEnvAccess, MemoryAdapter};
use axonyx_runtime::native::{AxNativeLimits, AxNativeRegistry};
use axonyx_runtime::server::AxHttpRequest;
use axonyx_runtime::{execute_preview_route_request_sources_with_runtime, AxPreviewStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const SOURCE: &str = include_str!("fixtures/native-backend.ax");

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Content {
    text: String,
}

fn runtime(limits: AxNativeLimits) -> AxDatabaseRuntime<MemoryAdapter> {
    let mut registry = AxNativeRegistry::new(limits);
    registry
        .register("content.prepare", |input: Content| {
            if input.text == "reject" {
                return Err("private callback diagnostic");
            }
            Ok(Content {
                text: input.text.trim().to_owned(),
            })
        })
        .unwrap();
    AxDatabaseRuntime::new(AxEnv::default(), MemoryAdapter).with_native_functions(registry)
}

#[test]
fn preview_and_native_runtime_use_the_same_typed_callback() {
    let runtime = runtime(AxNativeLimits::default());
    let response = execute_preview_route_request_sources_with_runtime(
        &[SOURCE],
        &AxHttpRequest::new("GET", "/prepare?text=%20hello%20"),
        &runtime,
        &mut AxPreviewStore::default(),
    )
    .unwrap()
    .unwrap();
    let expected = runtime
        .runtime_native_call("content.prepare", json!({"text":" hello "}))
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&response.body).unwrap(),
        expected
    );
    let module = compile_backend_ax_to_module(SOURCE).unwrap();
    assert!(module.contains("runtime.runtime_native_call(\"content.prepare\", json!("));
    assert!(!module.contains("Native::call"));
}

#[test]
fn preview_rejects_wrong_types_unknown_functions_and_private_errors() {
    let runtime = runtime(AxNativeLimits::default());
    for (target, message) in [
        ("/invalid", "invalid native function input"),
        ("/unknown", "unknown native function"),
        (
            "/prepare?text=reject",
            "native function rejected the operation",
        ),
    ] {
        let error = execute_preview_route_request_sources_with_runtime(
            &[SOURCE],
            &AxHttpRequest::new("GET", target),
            &runtime,
            &mut AxPreviewStore::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(message), "{error}");
        assert!(!error.contains("private callback diagnostic"));
        assert!(!error.contains("content.missing"));
    }
}

#[test]
fn unconfigured_runtimes_and_payload_limits_fail_closed() {
    let plain = AxDatabaseRuntime::new(AxEnv::default(), MemoryAdapter);
    assert!(plain
        .runtime_native_call("content.prepare", Value::Null)
        .unwrap_err()
        .to_string()
        .contains("unavailable"));
    let runtime = runtime(AxNativeLimits {
        input_bytes: 4,
        ..Default::default()
    });
    let error = execute_preview_route_request_sources_with_runtime(
        &[SOURCE],
        &AxHttpRequest::new("GET", "/prepare?text=hello"),
        &runtime,
        &mut AxPreviewStore::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("payload limit exceeded"));
}

#[test]
fn dynamic_names_invalid_names_and_arity_are_rejected_by_both_paths() {
    let runtime = runtime(AxNativeLimits::default());
    for args in [
        "request.query.name, {text: \"hello\"}",
        "\"content::prepare\", {text: \"hello\"}",
        "\"content.prepare\"",
        "\"content.prepare\", {}, {}",
    ] {
        let source = format!("route GET \"/prepare\" {{\n  data result = Native.call({args})\n  return json(result)\n}}");
        let error = compile_backend_ax_to_module(&source)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Native.call requires"), "{args}: {error}");
        assert!(execute_preview_route_request_sources_with_runtime(
            &[&source],
            &AxHttpRequest::new("GET", "/prepare"),
            &runtime,
            &mut AxPreviewStore::default(),
        )
        .unwrap_err()
        .to_string()
        .contains("Native.call requires"));
    }
    let helper = "export fn prepare(text: String) -> String {\n  data result = Native.call(\"content.prepare\", {text: text})\n  return text\n}";
    assert!(compile_backend_ax_to_module(helper)
        .unwrap_err()
        .to_string()
        .contains("Native.call requires"));
}
