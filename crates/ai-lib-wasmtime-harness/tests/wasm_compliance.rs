//! PT-073: load `ai_lib_wasm.wasm` in **wasmtime** and exercise `ailib_load_manifest` +
//! `ailib_build_chat_request` (same scenario as compliance `protocol_loading` + `message_building`).
//!
//! Note: compliance YAML checks manifests as untyped `serde_yaml::Value`; `ailib_load_manifest` parses
//! into [`ProtocolManifest`](ai_lib_core::protocol::ProtocolManifest) and needs the full required
//! metadata fields (`status`, `category`, `official_url`, `support_contact`, …). The inline manifest
//! below matches that shape.
//!
//! Run from workspace root:
//! `cargo build -p ai-lib-wasm --target wasm32-wasip1 --release`
//! `cargo test -p ai-lib-wasmtime-harness --test wasm_compliance`

use std::path::{Path, PathBuf};

use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

fn workspace_target_wasm() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../target/wasm32-wasip1/release/ai_lib_wasm.wasm")
}

/// Minimal v2-shaped manifest that deserializes to `ProtocolManifest` (stricter than compliance
/// `protocol_loading`, which only checks id / protocol_version / endpoint.base_url).
const WASM_MANIFEST_YAML: &str = r#"
id: qwen
protocol_version: "2.0"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "mailto:test@example.com"
endpoint:
  base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1"
capabilities:
  required: ["text", "streaming", "tools"]
  optional: []
"#;

fn copy_to_guest(
    store: &mut Store<WasiP1Ctx>,
    memory: &wasmtime::Memory,
    bytes: &[u8],
) -> (u32, u32) {
    const PAGE: u64 = 64 * 1024;
    let base = memory.data_size(&mut *store) as u64;
    let len = bytes.len() as u64;
    let end = base.checked_add(len).expect("guest memory size overflow");
    let pages_needed = end.div_ceil(PAGE);
    let cur_pages = memory.size(&mut *store);
    if pages_needed > cur_pages {
        memory
            .grow(&mut *store, pages_needed - cur_pages)
            .expect("grow guest memory for compliance payload");
    }
    memory
        .write(&mut *store, base as usize, bytes)
        .expect("write guest memory");
    (
        u32::try_from(base).expect("guest ptr fits u32"),
        u32::try_from(len).expect("guest length fits u32"),
    )
}

#[test]
fn wasmtime_protocol_loading_and_message_building() {
    let wasm_path = workspace_target_wasm();
    assert!(
        wasm_path.exists(),
        "missing {} — run: cargo build -p ai-lib-wasm --target wasm32-wasip1 --release",
        wasm_path.display()
    );

    let manifest_bytes = WASM_MANIFEST_YAML.trim_start().as_bytes();

    let engine = Engine::default();
    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    preview1::add_to_linker_sync(&mut linker, |s| s).expect("link wasi preview1");

    let module = Module::from_file(&engine, &wasm_path).expect("load wasm module");
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .build_p1();
    let mut store = Store::new(&engine, wasi);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiate wasm");

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("guest memory export");

    let load = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, "ailib_load_manifest")
        .expect("ailib_load_manifest");
    let (ptr, len) = copy_to_guest(&mut store, &memory, &manifest_bytes);
    let handle = load
        .call(&mut store, (ptr, len))
        .expect("ailib_load_manifest call");
    assert!(handle != 0, "ailib_load_manifest returned 0 (failure)");

    let req_json = br#"{"model":"qwen-turbo","messages":[{"role":"user","content":"hello"}],"stream":false}"#;
    let build = instance
        .get_typed_func::<(u32, u32, u32), i32>(&mut store, "ailib_build_chat_request")
        .expect("ailib_build_chat_request");
    let (jptr, jlen) = copy_to_guest(&mut store, &memory, req_json);
    let rc = build
        .call(&mut store, (handle, jptr, jlen))
        .expect("ailib_build_chat_request call");
    assert_eq!(rc, 0, "ailib_build_chat_request expected 0, got {}", rc);

    let out_len = instance
        .get_typed_func::<(), u32>(&mut store, "ailib_out_len")
        .expect("ailib_out_len");
    let n = out_len.call(&mut store, ()).expect("ailib_out_len");
    assert!(n > 0, "expected non-empty request JSON in ailib_out");

    let out_ptr_fn = instance
        .get_typed_func::<(), u32>(&mut store, "ailib_out_ptr")
        .expect("ailib_out_ptr");
    let op = out_ptr_fn.call(&mut store, ()).expect("ailib_out_ptr");
    assert!(op != 0, "ailib_out_ptr returned null");

    let mut body = vec![0u8; n as usize];
    memory
        .read(&store, op as usize, &mut body)
        .expect("read ailib_out");
    let parsed: serde_json::Value =
        serde_json::from_slice(&body).expect("built request must be JSON");
    assert!(
        parsed.get("messages").is_some(),
        "built body should contain messages: {}",
        String::from_utf8_lossy(&body)
    );
}
