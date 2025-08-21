// Simple procedural fluid-like background.
// Creates a swirling color field using layered sin/cos waves advected over time.

struct FluidData {
    v0: vec4<f32>, // (window_size.x, window_size.y, time, scale)
    v1: vec4<f32> // (intensity, reserved1, reserved2, reserved3)
};

@group(2) @binding(0)
var<uniform> fluid: FluidData;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(position.xy, 0.0, 1.0);
    let window_size = fluid.v0.xy;
    let half_size = window_size * 0.5;
    o.world = position.xy * half_size; // match world scaling similar to grid
    return o;
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let x = fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
    let y = fract(sin(dot(p, vec2<f32>(269.5, 183.3))) * 43758.5453);
    return vec2<f32>(x,y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0,0.0));
    let c = hash2(i + vec2<f32>(0.0,1.0));
    let d = hash2(i + vec2<f32>(1.0,1.0));
    let u = f*f*(3.0 - 2.0*f);
    return mix(mix(a.x,b.x,u.x), mix(c.x,d.x,u.x), u.y);
}

@fragment
fn fragment(in: VOut) -> @location(0) vec4<f32> {
    let t = fluid.v0.z * 0.15;
    let window_size = fluid.v0.xy;
    let uv = in.world / max(window_size.x, window_size.y) * fluid.v0.w;
    // Curl-ish layers
    var v = vec2<f32>(uv.x + sin(uv.y*2.0 + t), uv.y + cos(uv.x*2.0 - t));
    var accum: f32 = 0.0;
    var amp: f32 = 0.5;
    var freq: f32 = 1.0;
    for (var i: i32 = 0; i < 5; i = i + 1) {
        accum = accum + noise(v * freq + t) * amp;
        freq = freq * 1.9;
        amp = amp * 0.55;
        v = v.yx + vec2<f32>(0.13, -0.11) * t;
    }
    let base = accum;
    let r = sin(base*6.283 + t*2.0)*0.5+0.5;
    let g = sin(base*6.283 + 2.094 + t*1.5)*0.5+0.5;
    let b = sin(base*6.283 + 4.188 + t)*0.5+0.5;
    let col = vec3<f32>(r,g,b);
    let finalc = mix(vec3<f32>(0.02,0.02,0.03), col, fluid.v1.x);
    return vec4<f32>(finalc, 1.0);
}
