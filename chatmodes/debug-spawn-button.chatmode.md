## debug-spawn-button

<!-- Purpose: Add a debug-only UI button (top-right) that spawns 100 additional balls when clicked. -->

<!-- Authority & Context Sourcing -->
<!-- Sources: src/gameplay/spawn/spawn.rs (ball spawning logic), src/debug/overlay.rs (debug UI overlay spawn patterns), src/debug/mod.rs (debug plugin system registration), Copilot instructions file for architectural conventions. -->

<instructions>
You WILL implement a feature gated (feature = "debug") UI button anchored in the top-right corner labeled "Spawn +100" that, when clicked, spawns 100 additional balls using existing palette & physics spawning conventions.

You MUST follow existing patterns from: 
 - debug overlay (absolute positioned UI Text using bevy::ui::Node with position_type: Absolute) 
 - ball spawning (spawn_ball_entity helper in gameplay::spawn::spawn.rs) 

You MUST avoid duplicating large spawning logic; you MAY adapt a slim inline loop invoking spawn_ball_entity. You MUST reuse CircleMesh resource if present and NOOP safely if absent (e.g., early return). You MUST randomize radius, color/material, and velocity consistent with config ranges (mirror logic in spawn_balls: radius_range, vel ranges, palette selection, restitution/friction/damping from config). Simpler velocity sampling is acceptable (uniform random within configured ranges) provided constraints remain respected.

You MUST confine all new code to debug-only compilation via #[cfg(feature = "debug")] wrappers (including new components/systems) to prevent release overhead.

You MUST add:
 1. A new Component marker: DebugSpawnButton.
 2. A modification to debug_overlay_spawn to also spawn the button entity (absolute positioned top-right). Style: minimal (background dark translucent, padding ~4px). Use a Button (bevy::ui::Button) + Text child label.
 3. A new system debug_spawn_button_interact that queries changed Interaction on the button and on Click spawns 100 balls.
 4. Registration of the new system inside DebugPlugin build() in the same Update set (DebugPreRenderSet) after existing overlay systems so it can rely on resources.

Button Layout Requirements:
 - position_type: Absolute
 - top: Val::Px(4.0)
 - right: Val::Px(6.0)
 - fixed size (optional) OR intrinsic; ensure text readable (font 14px); color white.
 - z-order should naturally appear above scene (UI usually overlays). Keep consistent layering (no extra styles unless needed).

Spawning Logic Requirements:
 - Count constant: 100 (define const DEBUG_EXTRA_SPAWN_COUNT: usize = 100; near system).
 - For each ball:
    * radius: if min<max choose rng.gen_range(min..max) else min.
    * position: You WILL spawn just outside the current view similar to ring approach OR (simpler) at random within a square spanning window extents * 0.5. Choose simpler approach but ensure no clustering at single point: uniform in [-w/2, w/2] x [-h/2, h/2].
    * velocity: sample independently vx in [vel_x_range.min, vel_x_range.max], vy in [vel_y_range.min, vel_y_range.max]; if both near zero clamp at subtle non-zero by reusing existing logic (max(1.0) safety not strictly needed but fine).
    * choose material & restitution identical technique to spawn_balls: if both palettes present pick random matching index; else fallback random color.
 - Maintain feature: if cfg.draw_circles false then no circle child (existing flag).

Safety & Performance:
 - Early return if CircleMesh resource missing (log once with info! or skip silently). DO NOT panic.
 - Avoid per-spawn allocations beyond what spawn_ball_entity inherently performs.
 - Use a single rng (thread_rng) reused inside loop.
 - Log a single info! summarizing number spawned.

Testing / Validation Instructions:
 - Add a cargo test (cfg(feature="debug")) optional; minimal requirement: run application with --features debug and click button: observe logs show +100 spawn and overlay stats ball_count increases by 100.
 - Manual smoke: Launch, verify button visible only in debug builds.

Implementation Steps (MANDATORY ORDER):
 1. In debug/overlay.rs (inside #[cfg(feature="debug")]) add new Component DebugSpawnButton.
 2. Modify debug_overlay_spawn to spawn button:
    commands.spawn((Button, DebugSpawnButton, bevy::ui::Node{ position_type: Absolute, top: Px(4), right: Px(6), ..Default::default() }, BackgroundColor(Color::srgba(0.05,0.05,0.08,0.6)), BorderRadius::all(Val::Px(4.0)),)).with_children(|p| { p.spawn((Text::new("Spawn +100"), TextFont{ font: font_handle.clone(), font_size:14.0, ..Default::default()}, TextColor(Color::WHITE))); });
 3. In debug/mod.rs add system debug_spawn_button_interact (behind cfg) registered in Update tuple inside DebugPreRenderSet.
 4. Implement system: query (&Interaction, &mut BackgroundColor), On Changed<Interaction>, highlight on Hover, revert on None, on Click call helper spawn_extra_balls(commands, resources...).
 5. Add helper function spawn_debug_extra_balls(...) colocated in debug/mod.rs (behind cfg) performing spawn loop using spawn_ball_entity.
 6. Ensure necessary use imports: rand::Rng; gameplay::spawn::spawn::{spawn_ball_entity, CircleMesh}; palettes; physics materials; GameConfig; components.
 7. Add const DEBUG_EXTRA_SPAWN_COUNT: usize = 100;.
 8. Add info!(target:"spawn", ...) log after spawning.

Acceptance Criteria:
 - Debug build shows button top-right.
 - Clicking button once increases ball count by exactly 100 (verify via overlay stats). Repeated clicks cumulative.
 - Release (no debug feature) build unaffected (no compilation errors; button code excluded).
 - No panics if clicked early before palettes loaded (button system runs after initial spawn; but still early-return safe if resources missing).

Prohibited:
 - Adding new dependencies beyond already present rand.
 - Spawning inside a per-frame system without a click interaction.
 - Panicking or unwrap() on missing resources.

Provide Diff Hints (EXAMPLE – DO NOT hard-code paths in final output):
 // In overlay.rs: add component + modify spawn function.
 // In debug/mod.rs: add systems & helper.

You MUST keep existing overlay behavior unchanged aside from added button.

Output: Return only the modified Rust code snippets (no extraneous commentary) when executing this prompt.
</instructions>

<validation>
You MUST confirm:
 - Component and system names exactly match those specified.
 - Button anchored correctly (top-right) with label text.
 - System registered in DebugPreRenderSet.
 - Spawning logic compiled (no missing imports) and uses spawn_ball_entity.
 - Logging target present.
</validation>

<!-- End of prompt -->