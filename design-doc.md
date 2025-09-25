<!-- Concise ~100 line game design (no technical implementation). Detailed prior drafts live in git history. -->

# Color Fusion – Compact Game Design (~100 lines)

## 1. Core Vision
Fast, readable, minimalist arcade loop: energetic blobs (metaballs) ricochet in a contained arena, shatter fragile targets, and risk elimination by simple hazards. The joy comes from emergent trajectories, chaining hits, and tension as remaining balls dwindle.

## 2. Player Fantasy / Feel
Satisfying kinetic rebound (snappy, elastic).
Clear visual mass & momentum (squishy glow blobs).
Immediate comprehension: what can I hit, what should I avoid.
Short, restartable runs that invite “one more try”.

## 3. Core Loop (Player Perspective)
1. Observe arena layout (walls, targets, pit/hazard zones).
2. Launch / release initial balls (or they auto‑spawn). 
3. Watch / anticipate bounces; optionally influence future launches in later expansions (not in MVP). 
4. Targets struck: they pop / vanish, giving progress feedback.
5. Hazards remove balls; risk escalates as ball count drops.
6. All targets cleared (with ≥1 ball) -> win splash.
7. Last ball lost with targets remaining -> loss splash.
8. Quick restart; loop repeats.

## 4. Entities (Design Description)
Ball: mobile, high visibility, single uniform color (no mixing yet); conveys kinetic energy; primary agent of change.
Wall / Barrier: defines play bounds + optional interior angles to create interesting caroms; visually neutral to foreground balls/targets.
Target: one‑hit fragile object; provides audible/visual burst; reduction in remaining count is the core progress bar.
Hazard (Pit / Void / Kill Zone): negative space or marked region that deletes a ball; primary tension source; visually distinct edge or area.

## 5. Win / Lose Dynamics
Win Condition: 0 targets left, ≥1 ball.
Lose Condition: 0 balls left, ≥1 target.
Binary, immediate, no partial credit; emphasis on clarity over complexity.
Desirable pacing: average round length short (< 60s) to reinforce replayability.

## 6. Progress Feedback (Minimal)
Remaining targets count (numeric or shrinking cluster of icons).
Balls remaining (small pips or simple number). 
Transient hit flash + tiny screen pulse on last target.
Win: brief celebratory color flare. Lose: subdued fade.

## 7. Aesthetic Principles
Minimal palette: dark calming background, bright emissive balls, moderately bright targets, hazard accented in warning hue.
Readable silhouette at a glance; avoid overdrawn effects that obscure trajectory.
Particles: very sparing micro burst on target destruction (optional toggled effect).

## 8. Difficulty & Tuning Levers
Ball speed range (too slow = dull, too fast = unreadable).
Number of balls at start (more = easier early, harder to track late).
Target placement density (central cluster vs perimeter ring).
Hazard area size / placement (edge only vs interior pit).
Arena shape (pure rectangle vs angled pockets) for trajectory diversity.

## 9. Player Skill Expression (Even in Minimal Scope)
Predictive anticipation of rebound angles.
Strategic early target prioritization (remove those guarding safe corridors first).
Risk management: allow a ball to approach hazard to line up multi‑target chain vs conservative avoidance.

## 10. Audio / Feel Hooks (Placeholder Level Only)
Pop (target), thud (wall), soft dissolve (hazard removal) — all short, non‑intrusive, distinct envelope shapes. (Full soundscape deferred; described here only to guide feel.)

## 11. Scope Guardrails (Deliberately Excluded For Now)
No paddles/flippers, no scoring system, no color states or mixing, no ball merging/splitting, no switches, no portals, no moving platforms, no timers, no powerups, no progression meta, no menus beyond restart, no narrative, no multiplayer.

## 12. Near‑Term Extension Hooks (Design Outlook Only)
Add paddle to introduce agency.
Introduce color‑gated targets (simple two‑color first).
Layer score combos (streaks without hazard loss).
Each added only after core loop “feels” compelling.

## 13. Success Criteria (Design Centric)
Clarity: new observer understands objective inside 5 seconds.
Retention: average of multiple rapid replays in a session (compulsion loop intact).
Feel: players describe collisions as “satisfying” not “floaty” or “unfair”.
Focus: playtests don’t request tutorial—rules self evident.

## 14. Mantra
“Bounce. Break. Breathe. Repeat.”

