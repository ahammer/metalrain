// fluid_sim.wgsl - Minimal 2D Stable Fluids style compute passes + display.
// Additions: metaball / physics ball coupling via inject_balls pass (binding 8 storage buffer) which
// blends per-ball velocity & dye into the simulation BEFORE velocity advection. This is a naive O(N_cells * N_balls)
// approach adequate for small grids; optimize later with splatting per ball or tile culling.

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
    ball_count: u32,
    frame: u32,
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
// Ball injection buffer (array length fixed in Rust; we use ball_count to gate loops)
struct BallInstance {
    grid_pos: vec2<f32>,
    grid_vel: vec2<f32>,
    radius: f32,
    vel_inject: f32,
    color: vec4<f32>,
};
@group(0) @binding(8) var<storage, read> balls: array<BallInstance>;

// Utility sampling (nearest) for velocity (packed in RG, BA unused)
fn read_velocity(coord: vec2<i32>) -> vec2<f32> {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let c = clamp(coord, vec2<i32>(0), maxc);
    let v = textureLoad(velocity_in, c);
    return v.xy;
}
fn write_velocity(coord: vec2<i32>, v: vec2<f32>) { textureStore(velocity_out, coord, vec4<f32>(v,0.0,0.0)); }

// Bilinear sample velocity at fractional grid position (grid space, not normalized)
fn sample_velocity(pos: vec2<f32>) -> vec2<f32> {
    let maxc_f = vec2<f32>(vec2<f32>(sim.grid_size) - vec2<f32>(1.0));
    let p = clamp(pos, vec2<f32>(0.0), maxc_f);
    let p0 = floor(p);
    let frac = p - p0;
    let i00 = vec2<i32>(p0);
    let i10 = i00 + vec2<i32>(1,0);
    let i01 = i00 + vec2<i32>(0,1);
    let i11 = i00 + vec2<i32>(1,1);
    let v00 = read_velocity(i00);
    let v10 = read_velocity(i10);
    let v01 = read_velocity(i01);
    let v11 = read_velocity(i11);
    let vx0 = mix(v00, v10, vec2<f32>(frac.x, frac.x));
    let vx1 = mix(v01, v11, vec2<f32>(frac.x, frac.x));
    return mix(vx0, vx1, vec2<f32>(frac.y, frac.y));
}

// Bilinear sample dye (scalar_a) at fractional grid position
fn sample_dye(pos: vec2<f32>) -> vec4<f32> {
    let maxc_f = vec2<f32>(vec2<f32>(sim.grid_size) - vec2<f32>(1.0));
    let p = clamp(pos, vec2<f32>(0.0), maxc_f);
    let p0 = floor(p);
    let frac = p - p0;
    let i00 = vec2<i32>(p0);
    let i10 = i00 + vec2<i32>(1,0);
    let i01 = i00 + vec2<i32>(0,1);
    let i11 = i00 + vec2<i32>(1,1);
    let c00 = textureLoad(scalar_a, i00);
    let c10 = textureLoad(scalar_a, i10);
    let c01 = textureLoad(scalar_a, i01);
    let c11 = textureLoad(scalar_a, i11);
    let cx0 = mix(c00, c10, vec4<f32>(frac.x));
    let cx1 = mix(c01, c11, vec4<f32>(frac.x));
    return mix(cx0, cx1, vec4<f32>(frac.y));
}

// Semi-Lagrangian advection (velocity field self-advection)
@compute @workgroup_size(8,8,1)
fn advect_velocity(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let g = vec2<f32>(f32(gid.x), f32(gid.y));
    let uv = read_velocity(gid);
    let back = g - uv * sim.dt; // fractional backtrace
    let samp = sample_velocity(back);
    let v_new = samp * sim.vel_dissipation; // dissipation
    write_velocity(gid, v_new);
}

// Compute divergence of velocity field -> store in divergence_tex
@compute @workgroup_size(8,8,1)
fn compute_divergence(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
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
fn load_pressure(c: vec2<i32>) -> f32 {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let cc = clamp(c, vec2<i32>(0), maxc);
    return textureLoad(pressure_in, cc).x;
}

@compute @workgroup_size(8,8,1)
fn jacobi_pressure(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pL = load_pressure(gid + vec2<i32>(-1,0));
    let pR = load_pressure(gid + vec2<i32>(1,0));
    let pD = load_pressure(gid + vec2<i32>(0,-1));
    let pU = load_pressure(gid + vec2<i32>(0,1));
    let div = textureLoad(divergence_tex, gid).x;
    let p_new = (pL + pR + pD + pU + div * sim.jacobi_alpha) * sim.jacobi_beta;
    textureStore(pressure_out, gid, vec4<f32>(p_new,0.0,0.0,0.0));
}

// Projection: subtract gradient of pressure from velocity
@compute @workgroup_size(8,8,1)
fn project_velocity(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pL = load_pressure(gid + vec2<i32>(-1,0));
    let pR = load_pressure(gid + vec2<i32>(1,0));
    let pD = load_pressure(gid + vec2<i32>(0,-1));
    let pU = load_pressure(gid + vec2<i32>(0,1));
    let vel = read_velocity(gid);
    let grad = vec2<f32>(pR - pL, pU - pD) * 0.5;
    write_velocity(gid, vel - grad);
}

// Advect dye using velocity (scalar_a -> scalar_b)
@compute @workgroup_size(8,8,1)
fn advect_dye(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let vel = read_velocity(gid);
    let g = vec2<f32>(f32(gid.x), f32(gid.y));
    let back = g - vel * sim.dt;
    let dye_sample = sample_dye(back);
    textureStore(scalar_b, gid, dye_sample * sim.dissipation);
}

// Simple force injection (adds radial impulse & dye at force_pos)
@compute @workgroup_size(8,8,1)
fn add_force(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
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

// Inject per-ball velocity (adds to velocity_out) and dye (writes to scalar_b) before advection.
// Strategy: Each invocation checks distance to each ball; naive O(N*M) but N=grid cells, M=balls (<=1024).
// Later optimization: splat each ball in its own pass or restrict loop radius.
@compute @workgroup_size(8,8,1)
fn inject_balls(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let pos = vec2<f32>(f32(gid.x), f32(gid.y));
    var vel = read_velocity(gid);
    var dye = textureLoad(scalar_a, gid); // pass-through (ball dye injection disabled)
    // Accumulate influences
    for (var i: u32 = 0u; i < sim.ball_count; i = i + 1u) {
        let b = balls[i];
        let d = pos - b.grid_pos;
        let r = b.radius;
        if (r <= 0.0) { continue; }
        let dist2 = dot(d,d);
        let r2 = r*r;
        if (dist2 > r2) { continue; }
        let falloff = 1.0 - dist2 / r2; // simple linear falloff inside radius
        // Velocity contribution: impulse toward ball velocity direction (swirl not applied here)
        vel += b.grid_vel * b.vel_inject * falloff * sim.dt;
        // Dye injection disabled: intentionally skip modifying dye
    }
    write_velocity(gid, vel);
    textureStore(scalar_b, gid, dye); // unchanged dye
}

// Display shader (sample dye texture) - material bind group (group=2 in Bevy 2D materials)
@group(2) @binding(0) var dye_tex: texture_2d<f32>;
@group(2) @binding(1) var dye_sampler: sampler;

struct VOutDisplay { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vertex(@location(0) position: vec3<f32>) -> VOutDisplay {
    var o: VOutDisplay;
    o.pos = vec4<f32>(position.xy, 0.0, 1.0);
    o.uv = position.xy * 0.5 + vec2<f32>(0.5,0.5);
    return o;
}
@fragment
fn fragment(in: VOutDisplay) -> @location(0) vec4<f32> {
    let c = textureSample(dye_tex, dye_sampler, in.uv);
    return vec4<f32>(c.rgb, 1.0);
}
