# Changelog

All notable changes to this project will be documented in this file.

## Unreleased
- Split clustering logic into `ClusterCorePlugin` and `ClusterDebugPlugin`; umbrella `ClusterPlugin` preserved for backwards compatibility.
- Removed legacy `ClusterPopConfig` fields: `tap_radius` and `aabb_pad` along with associated validation and debug overlay display.
- Converted picking to ball-first fields only: `ball_pick_radius`, `ball_pick_radius_scale_with_ball`, `prefer_larger_radius_on_tie` now the canonical config knobs.
- Deleted obsolete `interaction::input::input_interaction` stub module.
- Cleaned debug overlay: replaced deprecated `tapR` with `pickR`; removed unnecessary `#[allow(dead_code)]` attributes.
- Added WebGPU availability guard invocation at WASM startup (no-op on native) enforcing explicit WebGPU requirement.
- Added `BallState` (Enabled/Disabled) component + `BallStatePlugin` classifying clusters each frame (size & total area thresholds).
- Introduced secondary fixed color palette (`SECONDARY_COLORS`) with time-based tween between enabled/disabled variants.
- Disabled (non-poppable) clusters now allocate unique color slots per ball to prevent metaball field merging (clear visual separation).
- Config: added `ball_state.tween_duration` (warn & clamp logic via usage) with validation warning on non-positive values.
- Metaball material update refactored to support dual-mode coloring and overflow fallback when exceeding `MAX_CLUSTERS`.

## 0.1.0
- Initial public release (metaballs rendering, clustering, cluster pop interaction, surface noise, debug feature flag, config layering).
