# Refine Compute Metaball Pipeline Prompt

## Purpose
You WILL refactor metaball rendering into a low‑resolution compute accumulation + high‑resolution fragment shading pipeline that preserves crisp iso edges, adds cluster coloring, dominant contributor logic, and improved shadows while reducing per‑pixel cost.

## Goals
1. Offload heavy field + gradient + dominance + shadow work to a LowRes compute pass.
2. Keep silhouettes razor sharp at FullRes by reconstructing signed distance from upsampled field + gradient.
3. Provide cluster / ball driven color with stable dominance blending.
4. Support Phong-ish (spec + rim + fresnel) faux 3D lighting.
5. Allow scale factor (2/4/8) without blocky artifacts.

## Resolution Definitions
- FullRes: swapchain dimensions (W,H).
- LowRes: (W / SCALE, H / SCALE) with SCALE ∈ {2,4,8}. Hot‑reloadable.
- Coordinate mapping: uv_full == uv_low (same normalized space). Sampler modes differ (linear vs nearest).

## GPU Resources
Create (rebuilt when SCALE changes):
- FieldTex (LowRes, RGBA16F)
  - R: field F
  - G: grad.x (analytic)
  - B: grad.y
  - A: shadow_or_ao (0..1)
- IndexTex (LowRes, RGBA32Uint)
  - R: dominant_ball_index (0xFFFFFFFF if none)
  - G: dominant_cluster_id
  - B: secondary_cluster_id (0xFFFFFFFF if none)
  - A: packed_dominance (u32 = dominance_ratio * 1e6)
- Storage buffer: Balls[] (pos, radius, cluster_id, color_index)
- Storage buffer: Clusters[] (vec4 color, optional extra params) (read‑only)
- Uniform buffer (Params) shared by compute & fragment.

## Uniform Params Structure (example)
```
struct Params {
  iso: f32;
  edge_band: f32;
  normal_z: f32;
  height_scale: f32;
  height_sharp: f32;
  ambient: f32;
  spec_pow: f32;
  spec_int: f32;
  rim_pow: f32;
  rim_int: f32;
  fresnel_int: f32;
  outline_w: f32;
  outline_int: f32;
  glow_w: f32;
  glow_int: f32;
  light_dir: vec3<f32>; _pad0: f32;
  low_res_inv: vec2<f32>;
  full_res_inv: vec2<f32>;
  scale_factor: f32; shadow_steps: u32; shadow_step_scale: f32; shadow_int: f32;
  refine_threshold: f32; enable_refine: u32; mode: u32; _pad1: u32;
}
```
Add #[repr(C,align(16))] + ShaderType in Rust.

## Compute Pass (WGSL) Requirements
@compute workgroup_size(8,8,1) (adjust if profiling dictates).
Per LowRes pixel:
1. uv_low = (gid.xy + 0.5) * low_res_inv.
2. Initialize: F=0, G=vec2(0), best_val=0, second_val=0, best_ball=0xFFFFFFFF, best_cluster=0xFFFFFFFF, second_cluster=0xFFFFFFFF.
3. For i in 0..ball_count:
   - dp = uv_low - ball.pos
   - d2 = dot(dp,dp) + eps
   - val = kernel(ball.radius, d2) (current field contribution form; replicate existing logic)
   - F += val
   - G += analytic_grad(ball.radius, dp, d2)
   - Track dominant & second (update best/second with stable ordering)
4. Dominance ratio = best_val / max(best_val + second_val, 1e-6).
5. Optional directional shadow (mode dependent):
   - If mode >=1 perform N = shadow_steps iterations:
     * sd ≈ (F - iso)/max(length(G),1e-6)
     * step_len = clamp(sd * shadow_step_scale, min_step, max_step)
     * sample point advance along projected light_dir_screen (normalize light_dir.xy)
     * Accumulate occlusion = min(occlusion + f(sd), 1) (choose simple: occlusion = max(occlusion, smoothstep(0,1,-sd)))
     * Early exit if occlusion >= 0.99
   - shadow_factor = 1 - occlusion; store in A channel.
   - If mode==0 set shadow_factor=1.
6. Pack outputs:
   - FieldTex(x,y) = (F, G.x, G.y, shadow_factor)
   - IndexTex(x,y) = (best_ball, best_cluster, second_cluster, packed_dominance)

Performance MUST:
- Avoid branching inside inner ball loop beyond dominance updates.
- Consider splitting balls into tiles if count > threshold (future optimization, not mandatory now).

## Fragment Pass (WGSL) Requirements
1. Sample FieldTex with LINEAR filtering (one tap) at uv.
2. Sample IndexTex with NEAREST (integer IDs).
3. Unpack: F, Gx, Gy, shadow_fac, cluster IDs, dominance.
4. Compute g_len = max(length(G), 1e-6).
5. sd = (F - iso)/g_len.
6. Edge AA width w = edge_band * g_len * 0.5 * (full_res_inv.x + full_res_inv.y).
7. inside = smoothstep(iso - w, iso + w, F).
8. Optional refine (if enable_refine!=0 AND abs(sd)<refine_threshold):
   - Gather 4 neighboring FieldTex samples (linear or manual offset). Fit plane via central differences; recompute G, g_len, sd.
9. height shaping: h_raw = clamp(sd*height_scale, -1,1); h_curve = sign(h_raw)*pow(abs(h_raw), height_sharp); normal = normalize(vec3(-G.x, -G.y, normal_z + h_curve)).
10. Lighting: diffuse/spec/rim/fresnel as defined; spec uses reflect.
11. Cluster color:
    - Validate indices < cluster_count else fallback color.
    - dominance = packed / 1e6.
    - secondary fallback to dominant if invalid.
    - base_color = mix(secondary, dominant, dominance).
12. Glow: outside_sd = max(-sd,0); glow_profile = exp(-(outside_sd / glow_w)^2); glow_rgb = base_color * 0.5 * glow_profile * glow_int.
13. Outline: ow = outline_w * w; outline = 1 - smoothstep(0, ow, abs(sd)).
14. Composite background gradient + shadow_fac * shadow_int darkening.
15. Merge inside shading, glow, outline.
16. Output vec4(color,1).

## Samplers
- FieldTexSampler: linear clamp.
- IndexTexSampler: nearest clamp.

## Packing Helpers
- dominance pack: u32(dominance * 1e6) ; unpack = f32(u)/1e6.
- Invalid cluster sentinel: 0xFFFFFFFFu.

## Rust Integration Steps
1. Extend config with scale_factor, shadow params, refine params (serde default + validation).
2. Create LowRes textures each frame on resize or scale change.
3. Create pipelines: compute (bind groups: Balls, Clusters, Params, FieldTex/IndexTex writes), fragment material pipeline reading textures.
4. Scheduling: compute pass in its own system before sprite/2d rendering (e.g., in a PreRender set).
5. Hot reload: updating Params resource updates uniform buffer; no pipeline rebuild.
6. Debug overlay: show SCALE, average compute ms, number of balls, mode.
7. Optional side-by-side debug quad rendering raw F (grayscale) for QA.

## Validation / Metrics
You MUST log once on mode change:
- scale_factor
- compute_time_ms (averaged over 60 frames)
- fragment_time_ms (optional if available)

Success Criteria:
- Silhouette crispness preserved vs baseline (visual inspection & sd refine optional).
- GPU time improvement for high ball counts (document example: N balls: old X ms vs new Y ms).
- Stable cluster coloring (no flicker frame to frame when camera static).
- Shadow factor smooth (no blocky LowRes artifacts) at SCALE<=4; acceptable at SCALE=8 with refine enabled.

## Modes
- 0: Basic (no shadow, no refine)
- 1: Shadow
- 2: Shadow + Refine
- 3: Shadow + Refine + Fresnel (full)

## Edge Cases & Safeguards
- If ball_count == 0: write zeros; fragment outputs background.
- If dominance ratio NaN: set to 1.
- Clamp iso to [0.0001, 10].
- If scale_factor change fails texture allocation -> fallback to previous scale.

## Prohibited
- Recomputing per-ball contributions in fragment.
- Blending integer IDs.
- Using fwidth(F) for AA (must use gradient magnitude path).

## Deliverables
1. WGSL compute shader file (e.g., metaballs_compute.wgsl)
2. WGSL fragment shader file (e.g., metaballs_present.wgsl)
3. Rust plugin adding resources, creating textures, running compute each frame
4. Updated material / bind group layout definitions
5. Config + validation updates
6. Debug overlay additions

## Final Checklist
- [ ] Textures recreated on resize/scale change
- [ ] Linear vs nearest samplers correct
- [ ] Gradient & field linear filtered; IDs not filtered
- [ ] Dominance math stable (no divide by zero)
- [ ] Shadow respects mode switch
- [ ] Refine conditional only near edge
- [ ] No out-of-bounds cluster access
- [ ] Benchmarks captured

END OF PROMPT
