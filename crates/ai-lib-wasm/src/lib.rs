//! WASI `wasm32-wasip1` exports for protocol + driver helpers (PT-072 Phase 1).
//!
//! 中文：薄封装层，供 wasmtime 等加载；不携带 HTTP 客户端与策略（P 层）依赖。

use std::sync::Mutex;

use ai_lib_core::drivers::{create_driver, DriverResponse, ProviderDriver};
use ai_lib_core::error_code::StandardErrorCode;
use ai_lib_core::protocol::v2::capabilities::{CapabilitiesV2, Capability, LegacyCapabilities};
use ai_lib_core::protocol::v2::manifest::{ApiStyle, ManifestV2};
use ai_lib_core::protocol::{load_manifest_validated, ProtocolManifest, UnifiedRequest};
use ai_lib_core::types::message::Message;
use ai_lib_core::types::tool::ToolDefinition;
use serde::{Deserialize, Serialize};

/// JSON shape accepted by `ailib_build_chat_request` (omits `response_format` / full `UnifiedRequest` serde).
#[derive(Debug, Deserialize)]
struct WasmChatRequest {
    #[serde(default)]
    operation: String,
    model: String,
    messages: Vec<Message>,
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    tools: Option<Vec<ToolDefinition>>,
    #[serde(default)]
    tool_choice: Option<serde_json::Value>,
}

impl From<WasmChatRequest> for UnifiedRequest {
    fn from(w: WasmChatRequest) -> Self {
        UnifiedRequest {
            operation: w.operation,
            model: w.model,
            messages: w.messages,
            temperature: w.temperature,
            max_tokens: w.max_tokens,
            stream: w.stream,
            tools: w.tools,
            tool_choice: w.tool_choice,
            response_format: None,
        }
    }
}

static MANIFESTS: Mutex<Vec<Option<(ProtocolManifest, Vec<u8>)>>> = Mutex::new(Vec::new());
static LAST_OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static LAST_ERR: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn set_out(bytes: Vec<u8>) {
    *LAST_OUT.lock().expect("out lock") = bytes;
}

fn set_err(s: impl AsRef<str>) {
    *LAST_ERR.lock().expect("err lock") = s.as_ref().as_bytes().to_vec();
}

fn clear_err() {
    LAST_ERR.lock().expect("err lock").clear();
}

fn caps_from_manifest(m: &ProtocolManifest) -> Vec<Capability> {
    CapabilitiesV2::Legacy(LegacyCapabilities {
        streaming: m.capabilities.streaming,
        tools: m.capabilities.tools,
        vision: m.capabilities.vision,
        agentic: m.capabilities.agentic,
        reasoning: m.capabilities.reasoning,
        parallel_tools: m.capabilities.parallel_tools,
    })
    .all_capabilities()
}

fn api_style_from_raw(bytes: &[u8]) -> ApiStyle {
    serde_yaml::from_slice::<ManifestV2>(bytes)
        .map(|m| m.detect_api_style())
        .unwrap_or(ApiStyle::OpenAiCompatible)
}

fn driver_for_handle(handle: u32) -> Result<Box<dyn ProviderDriver>, String> {
    let g = MANIFESTS.lock().map_err(|e| e.to_string())?;
    let slot = handle
        .checked_sub(1)
        .and_then(|i| g.get(i as usize))
        .ok_or_else(|| "invalid manifest handle".to_string())?;
    let (m, raw) = slot.as_ref().ok_or_else(|| "invalid manifest handle".to_string())?;
    let caps = caps_from_manifest(m);
    let style = api_style_from_raw(raw);
    Ok(create_driver(style, m.id.as_str(), caps))
}

unsafe fn bytes_from_ptr(ptr: *const u8, len: usize) -> Result<&'static [u8], String> {
    if ptr.is_null() || len == 0 {
        return Err("null or empty input".to_string());
    }
    Ok(std::slice::from_raw_parts(ptr, len))
}

unsafe fn str_from_ptr(ptr: *const u8, len: usize) -> Result<String, String> {
    let b = bytes_from_ptr(ptr, len)?;
    std::str::from_utf8(b)
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

/// Returns manifest handle (1-based) or 0 on failure. Read `ailib_out_*` / `ailib_err_*`.
#[no_mangle]
pub unsafe extern "C" fn ailib_load_manifest(ptr: *const u8, len: usize) -> u32 {
    clear_err();
    let bytes = match bytes_from_ptr(ptr, len) {
        Ok(b) => b.to_vec(),
        Err(e) => {
            set_err(e);
            return 0;
        }
    };
    match load_manifest_validated(&bytes) {
        Ok(manifest) => {
            let mut g = match MANIFESTS.lock() {
                Ok(g) => g,
                Err(e) => {
                    set_err(e.to_string());
                    return 0;
                }
            };
            g.push(Some((manifest, bytes)));
            g.len() as u32
        }
        Err(e) => {
            set_err(e.to_string());
            0
        }
    }
}

/// 1 if supported, 0 if not, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn ailib_check_capability(
    handle: u32,
    name_ptr: *const u8,
    name_len: usize,
) -> i32 {
    clear_err();
    let name = match str_from_ptr(name_ptr, name_len) {
        Ok(s) => s,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    let g = match MANIFESTS.lock() {
        Ok(g) => g,
        Err(e) => {
            set_err(e.to_string());
            return -1;
        }
    };
    let slot = match handle
        .checked_sub(1)
        .and_then(|i| g.get(i as usize))
        .and_then(|x| x.as_ref())
    {
        Some((m, _)) => m,
        None => {
            set_err("invalid manifest handle");
            return -1;
        }
    };
    if slot.supports_capability(name.trim()) {
        1
    } else {
        0
    }
}

/// Build provider chat body JSON from manifest handle + UnifiedRequest-shaped JSON. Output in `ailib_out_*`.
#[no_mangle]
pub unsafe extern "C" fn ailib_build_chat_request(
    handle: u32,
    json_ptr: *const u8,
    json_len: usize,
) -> i32 {
    clear_err();
    let json_slice = match bytes_from_ptr(json_ptr, json_len) {
        Ok(b) => b,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    let req: UnifiedRequest = match serde_json::from_slice::<WasmChatRequest>(json_slice) {
        Ok(r) => r.into(),
        Err(e) => {
            set_err(format!("messages json: {}", e));
            return -1;
        }
    };
    let g = match MANIFESTS.lock() {
        Ok(g) => g,
        Err(e) => {
            set_err(e.to_string());
            return -1;
        }
    };
    let (m, raw) = match handle
        .checked_sub(1)
        .and_then(|i| g.get(i as usize))
        .and_then(|x| x.as_ref())
    {
        Some(x) => x,
        None => {
            set_err("invalid manifest handle");
            return -1;
        }
    };
    let driver = create_driver(api_style_from_raw(raw), m.id.as_str(), caps_from_manifest(m));
    let built = match driver.build_request(
        &req.messages,
        &req.model,
        req.temperature,
        req.max_tokens,
        req.stream,
        None,
    ) {
        Ok(r) => r,
        Err(e) => {
            set_err(e.to_string());
            return -1;
        }
    };
    match serde_json::to_vec(&built.body) {
        Ok(v) => {
            set_out(v);
            0
        }
        Err(e) => {
            set_err(e.to_string());
            -1
        }
    }
}

#[derive(Serialize)]
struct NormalizedResponse {
    content: Option<String>,
    finish_reason: Option<String>,
    usage: Option<serde_json::Value>,
    tool_calls: Vec<serde_json::Value>,
}

/// Parse provider response JSON using driver for this manifest. Output in `ailib_out_*`.
#[no_mangle]
pub unsafe extern "C" fn ailib_parse_chat_response(
    handle: u32,
    json_ptr: *const u8,
    json_len: usize,
) -> i32 {
    clear_err();
    let json_slice = match bytes_from_ptr(json_ptr, json_len) {
        Ok(b) => b,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    let body: serde_json::Value = match serde_json::from_slice(json_slice) {
        Ok(v) => v,
        Err(e) => {
            set_err(format!("response json: {}", e));
            return -1;
        }
    };
    let driver = match driver_for_handle(handle) {
        Ok(d) => d,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    let DriverResponse {
        content,
        finish_reason,
        usage,
        tool_calls,
        ..
    } = match driver.parse_response(&body) {
        Ok(r) => r,
        Err(e) => {
            set_err(e.to_string());
            return -1;
        }
    };
    let usage_v = usage.map(|u| serde_json::to_value(u).unwrap_or(serde_json::Value::Null));
    let norm = NormalizedResponse {
        content,
        finish_reason,
        usage: usage_v,
        tool_calls,
    };
    match serde_json::to_vec(&norm) {
        Ok(v) => {
            set_out(v);
            0
        }
        Err(e) => {
            set_err(e.to_string());
            -1
        }
    }
}

/// Classify HTTP error; writes `{"code":"E...."}` to `ailib_out_*`.
#[no_mangle]
pub unsafe extern "C" fn ailib_classify_error(
    status_code: u16,
    json_ptr: *const u8,
    json_len: usize,
) -> i32 {
    clear_err();
    let code = if json_len == 0 || json_ptr.is_null() {
        StandardErrorCode::from_http_status(status_code)
    } else {
        match bytes_from_ptr(json_ptr, json_len) {
            Ok(b) => match serde_json::from_slice::<serde_json::Value>(b) {
                Ok(v) => {
                    let class = v
                        .pointer("/error/type")
                        .or_else(|| v.get("type"))
                        .and_then(|x: &serde_json::Value| x.as_str())
                        .unwrap_or("");
                    let c = StandardErrorCode::from_error_class(class);
                    if c == StandardErrorCode::Unknown {
                        StandardErrorCode::from_http_status(status_code)
                    } else {
                        c
                    }
                }
                Err(_) => StandardErrorCode::from_http_status(status_code),
            },
            Err(_) => StandardErrorCode::from_http_status(status_code),
        }
    };
    let out = serde_json::json!({ "code": code.code() });
    match serde_json::to_vec(&out) {
        Ok(v) => {
            set_out(v);
            0
        }
        Err(e) => {
            set_err(e.to_string());
            -1
        }
    }
}

/// Extract usage object from response JSON. Output in `ailib_out_*` (may be `{}`).
#[no_mangle]
pub unsafe extern "C" fn ailib_extract_usage(json_ptr: *const u8, json_len: usize) -> i32 {
    clear_err();
    let json_slice = match bytes_from_ptr(json_ptr, json_len) {
        Ok(b) => b,
        Err(e) => {
            set_err(e);
            return -1;
        }
    };
    let body: serde_json::Value = match serde_json::from_slice(json_slice) {
        Ok(v) => v,
        Err(e) => {
            set_err(format!("response json: {}", e));
            return -1;
        }
    };
    let usage = body.get("usage").cloned().unwrap_or(serde_json::json!({}));
    match serde_json::to_vec(&usage) {
        Ok(v) => {
            set_out(v);
            0
        }
        Err(e) => {
            set_err(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn ailib_out_ptr() -> *const u8 {
    LAST_OUT
        .lock()
        .ok()
        .and_then(|g| {
            let g = &*g;
            if g.is_empty() {
                None
            } else {
                Some(g.as_ptr())
            }
        })
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub extern "C" fn ailib_out_len() -> usize {
    LAST_OUT.lock().map(|g| g.len()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn ailib_err_ptr() -> *const u8 {
    LAST_ERR
        .lock()
        .ok()
        .and_then(|g| {
            let g = &*g;
            if g.is_empty() {
                None
            } else {
                Some(g.as_ptr())
            }
        })
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub extern "C" fn ailib_err_len() -> usize {
    LAST_ERR.lock().map(|g| g.len()).unwrap_or(0)
}
