// WASM-only early guard to fail fast if WebGPU is unavailable.
// We intentionally DO NOT offer a WebGL (webgl) fallback: build requires navigator.gpu.
#[cfg(target_arch = "wasm32")]
pub fn assert_webgpu_available() {
    let win = web_sys::window().expect("no window");
    let nav = win.navigator();
    // Reflect::get returns Result<JsValue, JsValue>. Map the Ok value to a bool
    // and default to false on Err to avoid intermediate Option usage.
    let key = wasm_bindgen::JsValue::from_str("gpu");
    let has_gpu = js_sys::Reflect::get(&nav, &key)
        .map(|v| !v.is_undefined())
        .unwrap_or(false);
    if !has_gpu {
        panic!(
            "WebGPU (navigator.gpu) is required. Use a WebGPU-enabled browser (Chrome, Edge, Firefox Nightly w/ flag, or Safari Technology Preview). WebGL fallback intentionally disabled."
        );
    }
}
