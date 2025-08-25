// WASM-only early guard to fail fast if WebGPU is unavailable.
// We intentionally DO NOT offer a WebGL (webgl) fallback: build requires navigator.gpu.
#[cfg(target_arch = "wasm32")]
pub fn assert_webgpu_available() {
    let win = web_sys::window().expect("no window");
    let nav = win.navigator();
    let has_gpu = js_sys::Reflect::get(&nav, &wasm_bindgen::JsValue::from_str("gpu"))
        .ok()
        .filter(|v| !v.is_undefined())
        .is_some();
    if !has_gpu {
        panic!(
            "WebGPU (navigator.gpu) is required. Use a WebGPU-enabled browser (Chrome, Edge, Firefox Nightly w/ flag, or Safari Technology Preview). WebGL fallback intentionally disabled."
        );
    }
}

// No-op on native.
#[cfg(not(target_arch = "wasm32"))]
pub fn assert_webgpu_available() {}
