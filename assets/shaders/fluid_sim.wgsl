// fluid_sim_fixed.wgsl
// Complete WGSL version with solid-wall boundary handling,
// bilinear back-traced advection, and an extra enforce_boundaries pass.
// Work-group size: (8,8,1).  Pass order per frame:
//
// 1. add_force
// 2. advect_velocity
// 3. compute_divergence
// 4. jacobi_pressure   (iterate N times, ping-pong pressure_in / pressure_out)
// 5. project_velocity
// 6. enforce_boundaries        // ← new
// 7. advect_dye
//
// Velocity    : RG of rgba16float
// Divergence  : r16float (read-write)
// Pressure    : r16float
// Dye         : rgba8unorm

// ─────────────────────────────────────────────────────────────
// 0. Uniforms
// ─────────────────────────────────────────────────────────────
struct SimUniform {
    grid_size       : vec2<u32>,
    inv_grid_size   : vec2<f32>,
    dt              : f32,
    dye_dissipation : f32,
    vel_dissipation : f32,
    jacobi_alpha    : f32,
    jacobi_beta     : f32,
    force_pos       : vec2<f32>,
    force_radius    : f32,
    force_strength  : f32,
};
@group(0) @binding(0) var<uniform> sim : SimUniform;

// ─────────────────────────────────────────────────────────────
// 1. Storage / sampled textures
// ─────────────────────────────────────────────────────────────
@group(0) @binding(1) var velocity_in  : texture_storage_2d<rgba16float, read>;
@group(0) @binding(2) var velocity_out : texture_storage_2d<rgba16float, write>;

@group(0) @binding(3) var scalar_a : texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(4) var scalar_b : texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(5) var pressure_in  : texture_storage_2d<r16float, read>;
@group(0) @binding(6) var pressure_out : texture_storage_2d<r16float, write>;

@group(0) @binding(7) var divergence_tex : texture_storage_2d<r16float, read_write>;
// Phase 4 step 2: placeholder bindings for upcoming multi-impulse support (currently unused)
struct GpuImpulse { pos: vec2<f32>, radius: f32, strength: f32, dir: vec2<f32>, kind: u32, _pad: u32 };
@group(0) @binding(8) var<storage, read> impulses : array<GpuImpulse>;
@group(0) @binding(9) var<uniform> impulse_count : vec4<u32>; // x = count

// Phase 4: shared constants for impulse processing
const IMPULSE_FALLOFF_EXPONENT : f32 = 2.0;   // (1 - r/R)^n exponent
const DYE_INJECT_SCALE         : f32 = 0.15;  // global multiplier for dye deposition strength

// ─────────────────────────────────────────────────────────────
// 2. Velocity helpers
//    read_velocity : nearest sample with clamped addressing
//    read_velocity_lin : bilinear reconstruction for back-tracing
// ─────────────────────────────────────────────────────────────
fn read_velocity(coord : vec2<i32>) -> vec2<f32> {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let c    = clamp(coord, vec2<i32>(0), maxc);
    return textureLoad(velocity_in, c).xy;
}

fn read_velocity_lin(p : vec2<f32>) -> vec2<f32> {
    // manual bilinear sample in grid space
    let ip  = vec2<i32>(floor(p));
    let f   = p - vec2<f32>(ip);
    let v00 = read_velocity(ip);
    let v10 = read_velocity(ip + vec2<i32>(1,0));
    let v01 = read_velocity(ip + vec2<i32>(0,1));
    let v11 = read_velocity(ip + vec2<i32>(1,1));
    let vx0 = mix(v00, v10, f.x);
    let vx1 = mix(v01, v11, f.x);
    return mix(vx0, vx1, f.y);
}

fn write_velocity(coord : vec2<i32>, v : vec2<f32>) {
    textureStore(velocity_out, coord, vec4<f32>(v, 0.0, 0.0));
}

// ─────────────────────────────────────────────────────────────
// 3. apply_impulses – iterate all queued impulses adding velocity contributions
// ─────────────────────────────────────────────────────────────
@compute @workgroup_size(8,8,1)
fn apply_impulses(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    var v = read_velocity(gid);
    let cell_pos = vec2<f32>(vec2<i32>(gid));
    let count = impulse_count.x;
    if (count == 0u) { write_velocity(gid, v); return; }
    for (var i:u32 = 0u; i < count; i = i + 1u) {
        let imp = impulses[i];
        if (imp.strength <= 0.0 || imp.radius <= 0.0) { continue; }
        let d = cell_pos - imp.pos;
        let dist2 = dot(d,d);
        let r = imp.radius;
        if (dist2 < r * r) {
            let dist = sqrt(dist2);
            let norm_r = dist / r; // in [0,1)
            let falloff = pow(max(0.0, 1.0 - norm_r), IMPULSE_FALLOFF_EXPONENT);
            if (falloff > 0.0) {
                var dir = vec2<f32>(0.0,0.0);
                if (imp.kind == 0u) {
                    // Swirl: perpendicular to radial vector (normalized)
                    let safe_div = dist + 1e-5;
                    dir = vec2<f32>(-d.y, d.x) / safe_div;
                } else {
                    // Directional: provided dir (normalize to be safe)
                    let base = imp.dir;
                    let mag = max(length(base), 1e-5);
                    dir = base / mag;
                }
                // Scale by strength, falloff, and dt
                v += dir * imp.strength * falloff * sim.dt;
            }
        }
    }
    write_velocity(gid, v);
}

// ─────────────────────────────────────────────────────────────
// 4. advect_velocity – semi-Lagrangian self-advection (bilinear sample)
// ─────────────────────────────────────────────────────────────
@compute @workgroup_size(8,8,1)
fn advect_velocity(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    let p   = vec2<f32>(vec2<i32>(gid));
    let vel = read_velocity(gid);
    let backp = p - vel * sim.dt;               // back-trace in grid coords
    let samp  = read_velocity_lin(backp);
    write_velocity(gid, samp * sim.vel_dissipation);
}

// ─────────────────────────────────────────────────────────────
// 5. compute_divergence – central-difference divergence
// ─────────────────────────────────────────────────────────────
@compute @workgroup_size(8,8,1)
fn compute_divergence(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    let left  = read_velocity(gid + vec2<i32>(-1, 0));
    let right = read_velocity(gid + vec2<i32>( 1, 0));
    let down  = read_velocity(gid + vec2<i32>( 0,-1));
    let up    = read_velocity(gid + vec2<i32>( 0, 1));

    let div = ((right.x - left.x) + (up.y - down.y)) * 0.5;
    textureStore(divergence_tex, gid, vec4<f32>(div, 0.0, 0.0, 0.0));
}

// ─────────────────────────────────────────────────────────────
// 6. jacobi_pressure – one Jacobi relaxation iteration
// ─────────────────────────────────────────────────────────────
fn load_pressure(c : vec2<i32>) -> f32 {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let cc   = clamp(c, vec2<i32>(0), maxc);
    return textureLoad(pressure_in, cc).x;
}

@compute @workgroup_size(8,8,1)
fn jacobi_pressure(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    let pL  = load_pressure(gid + vec2<i32>(-1, 0));
    let pR  = load_pressure(gid + vec2<i32>( 1, 0));
    let pD  = load_pressure(gid + vec2<i32>( 0,-1));
    let pU  = load_pressure(gid + vec2<i32>( 0, 1));
    let div = textureLoad(divergence_tex, gid).x;

    let p_new = (pL + pR + pD + pU + div * sim.jacobi_alpha) * sim.jacobi_beta;
    textureStore(pressure_out, gid, vec4<f32>(p_new, 0.0, 0.0, 0.0));
}

// ─────────────────────────────────────────────────────────────
// 7. project_velocity – subtract ∇p to enforce incompressibility
// ─────────────────────────────────────────────────────────────
@compute @workgroup_size(8,8,1)
fn project_velocity(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    let pL  = load_pressure(gid + vec2<i32>(-1, 0));
    let pR  = load_pressure(gid + vec2<i32>( 1, 0));
    let pD  = load_pressure(gid + vec2<i32>( 0,-1));
    let pU  = load_pressure(gid + vec2<i32>( 0, 1));

    let vel  = read_velocity(gid);
    let grad = vec2<f32>(pR - pL, pU - pD) * 0.5;
    write_velocity(gid, vel - grad);
}

// ─────────────────────────────────────────────────────────────
// 8. enforce_boundaries – zero-normal velocity at domain edges
// ─────────────────────────────────────────────────────────────
@compute @workgroup_size(8,8,1)
fn enforce_boundaries(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    var v = read_velocity(gid);

    // West / East walls
    if (gid.x == 0 || gid.x == i32(sim.grid_size.x) - 1) {
        v.x = 0.0;              // no-slip; flip sign for slip wall
    }
    // South / North walls
    if (gid.y == 0 || gid.y == i32(sim.grid_size.y) - 1) {
        v.y = 0.0;
    }
    write_velocity(gid, v);
}

// ─────────────────────────────────────────────────────────────
// 9. advect_dye – move dye with final velocity (bilinear sample)
// ─────────────────────────────────────────────────────────────
fn read_dye(coord : vec2<i32>) -> vec4<f32> {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let c    = clamp(coord, vec2<i32>(0), maxc);
    return textureLoad(scalar_a, c);
}

fn read_dye_lin(p : vec2<f32>) -> vec4<f32> {
    let ip  = vec2<i32>(floor(p));
    let f   = p - vec2<f32>(ip);
    let d00 = read_dye(ip);
    let d10 = read_dye(ip + vec2<i32>(1,0));
    let d01 = read_dye(ip + vec2<i32>(0,1));
    let d11 = read_dye(ip + vec2<i32>(1,1));
    let dx0 = mix(d00, d10, f.x);
    let dx1 = mix(d01, d11, f.x);
    return mix(dx0, dx1, f.y);
}

@compute @workgroup_size(8,8,1)
fn advect_dye(@builtin(global_invocation_id) gid_in : vec3<u32>) {
    let gid = vec2<i32>(gid_in.xy);
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }

    let p   = vec2<f32>(vec2<i32>(gid));
    let vel = read_velocity(gid);
    let backp = p - vel * sim.dt;
    let dye   = read_dye_lin(backp);
    var out_dye = dye * sim.dye_dissipation;

    // Phase 4: dye deposition from impulses (uses same falloff as velocity injection)
    let count = impulse_count.x;
    if (count > 0u) {
        for (var i:u32 = 0u; i < count; i = i + 1u) {
            let imp = impulses[i];
            if (imp.strength <= 0.0 || imp.radius <= 0.0) { continue; }
            let d = p - imp.pos;
            let dist2 = dot(d,d);
            let r = imp.radius;
            if (dist2 < r * r) {
                let dist = sqrt(dist2);
                let norm_r = dist / r;
                let falloff = pow(max(0.0, 1.0 - norm_r), IMPULSE_FALLOFF_EXPONENT);
                if (falloff > 0.0) {
                    // Simple coloring: swirl (kind 0) = cool blue, directional = warm orange
                    let base_color = select(vec3<f32>(1.0, 0.55, 0.2), vec3<f32>(0.25, 0.6, 1.0), imp.kind == 0u);
                    let strength_scale = imp.strength * falloff * DYE_INJECT_SCALE;
                    out_dye = vec4<f32>(clamp(out_dye.rgb + base_color * strength_scale, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
                }
            }
        }
    }
    textureStore(scalar_b, gid, out_dye);
}

// ─────────────────────────────────────────────────────────────
// 10. Full-screen quad display program (unchanged)
// ─────────────────────────────────────────────────────────────
@group(2) @binding(0) var dye_tex   : texture_2d<f32>;
@group(2) @binding(1) var dye_sampler : sampler;

struct VOutDisplay {
    @builtin(position) pos : vec4<f32>,
    @location(0)       uv  : vec2<f32>,
};

@vertex
fn vertex(@location(0) position : vec3<f32>) -> VOutDisplay {
    var o : VOutDisplay;
    o.pos = vec4<f32>(position.xy, 0.0, 1.0);
    o.uv  = position.xy * 0.5 + vec2<f32>(0.5, 0.5);
    return o;
}

@fragment
fn fragment(in : VOutDisplay) -> @location(0) vec4<f32> {
    let c = textureSample(dye_tex, dye_sampler, in.uv);
    return vec4<f32>(c.rgb, 1.0);
}
