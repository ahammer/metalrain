// fluid_sim.wgsl - Minimal 2D Stable Fluids style compute passes + display.
// NOTE: First iteration keeps things deliberately simple; optimization & advanced advection can follow.

struct SimUniform {
    grid_size: vec2<u32>,
    inv_grid_size: vec2<f32>,
    dt: f32,
    dissipation: f32,
    vel_dissipation: f32,
    jacobi_alpha: f32,
    jacobi_beta: f32,
    force_pos: vec2<f32>,
    force_radius: f32,
    force_strength: f32,
}
@group(0) @binding(0) var<uniform> sim: SimUniform;

// Storage textures (declared as needed per entry point with matching bind groups set up in Rust)
// We will re-use the same binding indices across pipelines for simplicity.
@group(0) @binding(1) var velocity_in: texture_storage_2d<rgba16float, read>;
@group(0) @binding(2) var velocity_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var scalar_a: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(4) var scalar_b: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var pressure_in: texture_storage_2d<r16float, read>;
@group(0) @binding(6) var pressure_out: texture_storage_2d<r16float, write>;
// Needs read + write because we write in compute_divergence then read in jacobi_pressure
@group(0) @binding(7) var divergence_tex: texture_storage_2d<r16float, read_write>;

// Utility sampling (nearest) for velocity (packed in RG, BA unused)
fn read_velocity(coord: vec2<i32>) -> vec2<f32> {
    let v = textureLoad(velocity_in, coord, 0);
    return v.xy;
}
fn write_velocity(coord: vec2<i32>, v: vec2<f32>) { textureStore(velocity_out, coord, vec4<f32>(v,0.0,0.0)); }

// Semi-Lagrangian advection (velocity field self-advection)
@compute @workgroup_size(8,8,1)
fn advect_velocity() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let g = vec2<f32>(f32(gid.x), f32(gid.y));
    let uv = read_velocity(gid);
    let back = g - uv * sim.dt; // backtrace in grid space (not normalized to [0,1])
    let back_i = vec2<i32>(clamp(back, vec2<f32>(0.0), vec2<f32>(sim.grid_size) - vec2<f32>(1.0)));
    let samp = read_velocity(back_i);
    // Simple dissipation
    let v_new = samp * sim.vel_dissipation;
    write_velocity(gid, v_new);
}

// Compute divergence of velocity field -> store in divergence_tex
@compute @workgroup_size(8,8,1)
fn compute_divergence() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let left = read_velocity(gid + vec2<i32>(-1,0));
    let right = read_velocity(gid + vec2<i32>(1,0));
    let down = read_velocity(gid + vec2<i32>(0,-1));
    let up = read_velocity(gid + vec2<i32>(0,1));
    let hx = 0.5; // grid spacing assumed 1
    let div = ((right.x - left.x) + (up.y - down.y)) * 0.5; // approximate divergence
    textureStore(divergence_tex, gid, vec4<f32>(div,0.0,0.0,0.0));
}

// Jacobi pressure iteration: pressure_out = (divergence + (pL+pR+pU+pD)*alpha) * beta
@compute @workgroup_size(8,8,1)
fn jacobi_pressure() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pL = textureLoad(pressure_in, gid + vec2<i32>(-1,0), 0).x;
    let pR = textureLoad(pressure_in, gid + vec2<i32>(1,0), 0).x;
    let pD = textureLoad(pressure_in, gid + vec2<i32>(0,-1), 0).x;
    let pU = textureLoad(pressure_in, gid + vec2<i32>(0,1), 0).x;
    let div = textureLoad(divergence_tex, gid, 0).x;
    let p_new = (pL + pR + pD + pU + div * sim.jacobi_alpha) * sim.jacobi_beta;
    textureStore(pressure_out, gid, vec4<f32>(p_new,0.0,0.0,0.0));
}

// Projection: subtract gradient of pressure from velocity
@compute @workgroup_size(8,8,1)
fn project_velocity() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pL = textureLoad(pressure_in, gid + vec2<i32>(-1,0), 0).x;
    let pR = textureLoad(pressure_in, gid + vec2<i32>(1,0), 0).x;
    let pD = textureLoad(pressure_in, gid + vec2<i32>(0,-1), 0).x;
    let pU = textureLoad(pressure_in, gid + vec2<i32>(0,1), 0).x;
    let vel = read_velocity(gid);
    let grad = vec2<f32>(pR - pL, pU - pD) * 0.5;
    write_velocity(gid, vel - grad);
}

// Advect dye using velocity (scalar_a -> scalar_b)
@compute @workgroup_size(8,8,1)
fn advect_dye() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let vel = read_velocity(gid);
    let g = vec2<f32>(f32(gid.x), f32(gid.y));
    let back = g - vel * sim.dt;
    let back_i = vec2<i32>(clamp(back, vec2<f32>(0.0), vec2<f32>(sim.grid_size) - vec2<f32>(1.0)));
    let dye = textureLoad(scalar_a, back_i, 0);
    // Apply global dissipation
    textureStore(scalar_b, gid, dye * sim.dissipation);
}

// Simple force injection (adds radial impulse & dye at force_pos)
@compute @workgroup_size(8,8,1)
fn add_force() {
    let gid = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pos = vec2<f32>(f32(gid.x), f32(gid.y));
    let d = pos - sim.force_pos;
    let r2 = sim.force_radius * sim.force_radius;
    let dist2 = dot(d,d);
    if (dist2 < r2) {
        let falloff = 1.0 - dist2 / r2;
        let dir = normalize(vec2<f32>(-d.y, d.x)); // swirl
        let vel = read_velocity(gid) + dir * sim.force_strength * falloff * sim.dt;
        write_velocity(gid, vel);
        // inject color (scalar_b reused if bound suitably) - optional handled in dye pass for simplicity
    }
}

// Display shader (sample dye texture)
@group(1) @binding(0) var dye_tex: texture_2d<f32>;
@group(1) @binding(1) var dye_sampler: sampler;

struct VOutDisplay { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn display_vertex(@location(0) position: vec3<f32>) -> VOutDisplay {
    var o: VOutDisplay;
    o.pos = vec4<f32>(position.xy, 0.0, 1.0);
    o.uv = position.xy * 0.5 + vec2<f32>(0.5,0.5);
    return o;
}
@fragment
fn display_fragment(in: VOutDisplay) -> @location(0) vec4<f32> {
    let c = textureSample(dye_tex, dye_sampler, in.uv);
    return vec4<f32>(c.rgb, 1.0);
}
