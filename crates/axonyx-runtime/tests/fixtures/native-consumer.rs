mod backend;

use axonyx_runtime::backend::{AxDatabaseRuntime, AxEnv, MemoryAdapter};
use axonyx_runtime::native::{AxNativeLimits, AxNativeRegistry};
use axonyx_runtime::server::{AxHttpRequest, AxUnavailableFileStorage};
use axonyx_runtime::{
    execute_preview_action_request_sources_with_runtime_and_storage,
    execute_preview_route_request_sources_with_runtime, AxPreviewStore,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Content {
    text: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    items: Vec<Content>,
    #[serde(rename = "meta:kind")]
    kind: String,
}

fn runtime(limits: AxNativeLimits) -> AxDatabaseRuntime<MemoryAdapter> {
    let mut native = AxNativeRegistry::new(limits);
    native
        .register("content.prepare", |input: Content| {
            if input.text == "reject" {
                return Err("private diagnostic");
            }
            Ok(Content {
                text: input.text.trim().to_owned(),
            })
        })
        .unwrap();
    native
        .register("content.batch", |input: Batch| Ok::<_, ()>(input))
        .unwrap();
    AxDatabaseRuntime::new(AxEnv::default(), MemoryAdapter).with_native_functions(native)
}

fn main() {
    let source = include_str!("native-backend.ax");
    let runtime = runtime(AxNativeLimits::default());
    let request = AxHttpRequest::new("GET", "/prepare?t%65xt=%20hello%20");
    let compiled = backend::dispatch_api_route(&runtime, &request, false)
        .unwrap()
        .unwrap();
    let preview = execute_preview_route_request_sources_with_runtime(
        &[source],
        &request,
        &runtime,
        &mut AxPreviewStore::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(compiled.status, preview.status);
    assert_eq!(compiled.body.into_bytes(), preview.body);
    for request in [
        AxHttpRequest::new("GET", "/nested?text=hello"),
        AxHttpRequest::new("GET", "/members?text=hello"),
        AxHttpRequest::new("GET", "/prepare?text=old&text=hello+world"),
        AxHttpRequest::new("POST", "/form").with_body(b"text=hello".to_vec()),
    ] {
        let compiled = backend::dispatch_api_route(&runtime, &request, false)
            .unwrap()
            .unwrap();
        let preview = execute_preview_route_request_sources_with_runtime(
            &[source],
            &request,
            &runtime,
            &mut AxPreviewStore::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(compiled.status, preview.status);
        assert_eq!(compiled.body.into_bytes(), preview.body);
    }
    let members = backend::dispatch_api_route(&runtime, &AxHttpRequest::new("GET", "/members?text=hello"), false)
        .unwrap().unwrap();
    let members: serde_json::Value = serde_json::from_slice(&members.body.into_bytes()).unwrap();
    assert_eq!(members, serde_json::json!({"text":"hello", "grouped":"hello", "missing":null, "literal":"prepared.items[0].text"}));
    for target in ["/invalid", "/unknown", "/prepare?text=reject"] {
        let request = AxHttpRequest::new("GET", target);
        let compiled = backend::dispatch_api_route(&runtime, &request, false)
            .unwrap_err()
            .to_string();
        let preview = execute_preview_route_request_sources_with_runtime(
            &[source],
            &request,
            &runtime,
            &mut AxPreviewStore::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(preview.contains(&compiled), "{preview} != {compiled}");
        assert!(!compiled.contains("private diagnostic"));
        assert!(!compiled.contains("content.missing"));
    }
    let limited = self::runtime(AxNativeLimits {
        input_bytes: 4,
        ..Default::default()
    });
    assert!(backend::dispatch_api_route(&limited, &request, false)
        .unwrap_err()
        .to_string()
        .contains("payload limit exceeded"));
    let absent = AxDatabaseRuntime::new(AxEnv::default(), MemoryAdapter);
    assert!(backend::dispatch_api_route(&absent, &request, false)
        .unwrap_err()
        .to_string()
        .contains("unavailable"));
    let request = AxHttpRequest::new("POST", "/prepare").with_body(b"text=%20hello%20".to_vec());
    let compiled = backend::dispatch_action(
        &runtime,
        &AxUnavailableFileStorage,
        "PrepareContent",
        &request,
    )
    .unwrap()
    .unwrap();
    let preview = execute_preview_action_request_sources_with_runtime_and_storage(
        &[source],
        "PrepareContent",
        &request,
        &runtime,
        &AxUnavailableFileStorage,
        &mut AxPreviewStore::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.payload["value"],
        serde_json::json!({"text":"hello"})
    );
    assert!(preview.error.is_none());
    assert_eq!(
        preview.value,
        axonyx_core::ax_lowering::AxValue::record([(
            "text",
            axonyx_core::ax_lowering::AxValue::from("hello")
        ),])
    );
    for (body, valid) in [(b"text=hello".as_slice(), true), (b"text=%20".as_slice(), false)] {
        let request = AxHttpRequest::new("POST", "/check").with_body(body.to_vec());
        let compiled = backend::dispatch_action(&runtime, &AxUnavailableFileStorage, "CheckMember", &request)
            .unwrap().unwrap();
        let preview = execute_preview_action_request_sources_with_runtime_and_storage(
            &[source], "CheckMember", &request, &runtime, &AxUnavailableFileStorage,
            &mut AxPreviewStore::default()
        ).unwrap();
        if valid {
            assert_eq!(compiled.payload["value"], "hello");
            assert!(preview.error.is_none());
            assert_eq!(preview.value, axonyx_core::ax_lowering::AxValue::from("hello"));
        } else {
            assert_eq!(compiled.payload["error"]["status"], 422);
            assert_eq!(compiled.payload["error"]["value"]["fields"]["text"], "Required");
            assert_eq!(preview.error.unwrap().status, 422);
        }
    }
    println!("Native calls: generated Rust and preview parity passed");
}
