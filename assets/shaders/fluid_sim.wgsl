// fluid_sim.wgsl - 2D Stable Fluids style simulation + fullscreen display.
// Pass order each frame (Rust orchestrated):
//   1. add_force          : injects user-driven swirl impulse into velocity (in->out)
//   2. advect_velocity    : semi-lagrangian self-advection with dissipation (in->out)
//   3. compute_divergence : divergence of current velocity written to divergence_tex
//   4. jacobi_pressure    : N Jacobi iterations (ping-pong pressure_in/out) solving Poisson eq.
//   5. project_velocity   : subtract pressure gradient from velocity (enforces near incompressibility)
//   6. advect_dye         : move dye using final velocity, apply dissipation (scalar_a->scalar_b)
// After certain passes Rust copies back *b -> *a to keep stable handles for display.
// Simplifications:
//   * Velocity stored in RG of RGBA16F; BA unused.
//   * Pressure, divergence use R16F.
//   * No boundary conditions beyond simple clamping (acts like solid walls); can be extended.
//   * No vorticity confinement or MacCormack; semi-Lagrangian is diffusive but stable.
//   * Force is a tangential swirl for visually pleasing motion.
// Workgroup size is (8,8,1); Rust dispatch rounds up to cover the grid.

// Simulation parameters (std140-style padded to 64B on Rust side).
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
// Nearest neighbor velocity fetch with clamped addressing.
fn read_velocity(coord: vec2<i32>) -> vec2<f32> {
    let maxc = vec2<i32>(vec2<i32>(sim.grid_size) - vec2<i32>(1));
    let c = clamp(coord, vec2<i32>(0), maxc);
    let v = textureLoad(velocity_in, c);
    return v.xy;
}
fn write_velocity(coord: vec2<i32>, v: vec2<f32>) { textureStore(velocity_out, coord, vec4<f32>(v,0.0,0.0)); }

// Semi-Lagrangian advection (velocity field self-advection)
// Velocity self-advection: backtrace along velocity and sample prior field.
@compute @workgroup_size(8,8,1)
fn advect_velocity(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
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
// Divergence = dUx/dx + dVy/dy (central differences). Stored for pressure solve.
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

// One Jacobi relaxation step toward solving ∇^2 p = divergence.
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
// Projection: vel' = vel - grad(p).
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
// Dye advection identical pattern to velocity but samples dye scalar (rgba8) with dissipation.
@compute @workgroup_size(8,8,1)
fn advect_dye(@builtin(global_invocation_id) gid_in: vec3<u32>) {
    let gid = vec2<i32>(i32(gid_in.x), i32(gid_in.y));
    if (u32(gid.x) >= sim.grid_size.x || u32(gid.y) >= sim.grid_size.y) { return; }
    let vel = read_velocity(gid);
    let g = vec2<f32>(f32(gid.x), f32(gid.y));
    let back = g - vel * sim.dt;
    let back_i = vec2<i32>(clamp(back, vec2<f32>(0.0), vec2<f32>(sim.grid_size) - vec2<f32>(1.0)));
    let dye = textureLoad(scalar_a, back_i);
    // Apply global dissipation
    textureStore(scalar_b, gid, dye * sim.dissipation);
}

// Simple force injection (adds radial impulse & dye at force_pos)
// Force injection: swirl impulse inside a radius around sim.force_pos.
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
