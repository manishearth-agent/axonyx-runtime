//! Shared validation for explicit native backend calls.

use crate::ax_backend_lowering::AxRustExpr;

pub fn is_native_function_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
}

/// Only a source-owned literal name and one payload are accepted.
pub fn native_call_name(args: &[AxRustExpr]) -> Option<String> {
    let [name, _] = args else { return None };
    let code = name.code.trim();
    let literal = code.strip_suffix(".to_string()").unwrap_or(code);
    let name: String = serde_json::from_str(literal).ok()?;
    is_native_function_name(&name).then_some(name)
}
