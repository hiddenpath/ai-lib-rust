# PT-073 / Wave-5: smoke-check ai-lib-wasm exports with wasmtime CLI.
# Prerequisites: wasmtime on PATH, rust wasm32-wasip1 target.
# Usage (from repo root): pwsh -File scripts/wasmtime-pt073-smoke.ps1

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$WasmPath = Join-Path $RepoRoot "target/wasm32-wasip1/release/ai_lib_wasm.wasm"

Push-Location $RepoRoot
try {
    if (-not (Test-Path $WasmPath)) {
        Write-Host "Building ai-lib-wasm (release)..."
        cargo build -p ai-lib-wasm --target wasm32-wasip1 --release
    }

    if (-not (Get-Command wasmtime -ErrorAction SilentlyContinue)) {
        Write-Warning "wasmtime not found on PATH; skip invoke smoke."
        exit 0
    }

    # Sanity: exported functions return without trapping (len 0 when buffers empty).
    wasmtime run $WasmPath --invoke ailib_out_len | Out-Null
    wasmtime run $WasmPath --invoke ailib_err_len | Out-Null
    Write-Host "PT-073 wasmtime smoke: OK ($WasmPath)"
}
finally {
    Pop-Location
}
