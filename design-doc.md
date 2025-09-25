<!--
NOTE: This document was aggressively reduced on 2025-09-25 to reflect a *Minimum Viable Prototype* (MVP) scope.
Anything not explicitly listed as IN-SCOPE is OUT-OF-SCOPE for the first playable.
The previous expansive design (pinball/breakout hybrid with color mixing, portals, complex widgets, multi-phase puzzles)
is intentionally archived in git history and can be re‑introduced iteratively.
-->

# Color Fusion – MVP Design (Slim Scope)

## 0. Goals of the MVP
Deliver a tight, performant core that proves: (1) metaball visual + (2) responsive 2D physics + (3) simple interaction loop (hit targets, avoid hazards). Everything else waits.

Success Criteria:
* Stable 60 FPS (desktop) with a target of 150 active balls (stress scene) using current GPU field technique.
* Deterministic physics step without observable tunneling for nominal ball speeds.
* Clear win condition (all targets destroyed) and fail condition (all balls lost / hazard depletion) with immediate restart.
* Code structured so adding future mechanics (color, gates, multi-ball gadgets) does NOT require major refactors.

## 1. Core Game Loop (MVP)
1. Spawn one or more balls inside an arena.
2. Balls move under physics (Rapier 2D) with elastic collisions against static barriers.
3. Player (initially) only influences the simulation indirectly (e.g. optional single “launcher impulse” or nudge – may even be deferred to stretch).
4. Balls strike breakable targets; each target on hit: play FX -> mark destroyed -> remove collider/visual.
5. Hazards (pit, spike, field) remove or “consume” a ball.
6. Win: all targets destroyed while at least one ball remains. Lose: zero balls remain while targets still exist.
7. Show simple overlay (Win / Lose) and allow instant reset (R / Space).

Out of loop for MVP: scoring system, combo chains, UI HUD beyond minimalist counters.

## 2. Entity & System Inventory (In-Scope Only)

| Category | Purpose | Notes |
|----------|---------|-------|
| Ball | Dynamic physics body + metaball render sample | No merging, no splitting, no color states beyond a single uniform color set / palette index. |
| Barrier | Static collider (walls, angled deflectors) | Author via simple level asset or inline spawn list. |
| Target | Breakable collider; one hit destroys | Optional hit flash, optional debris particles (feature flag). |
| Hazard: Pit | Out-of-bounds trigger region | Despawns ball immediately. |
| Hazard: Spike / Kill Volume | On contact remove ball | Could reuse sensor collider with event. |
| Metaball Field | GPU density field + normal compute + fullscreen present | Already partially implemented in `metaball_renderer`. |
| Game State | Resource tracking counts (balls, remaining targets) | Drives win/lose evaluation system. |

Out-of-scope (MVP): color mixing, portals, switches, gates, flippers/paddles, multi-ball powerups, score, timers, bosses, progression, UI menus, audio system, save data.

## 3. Rendering (MVP)
* Reuse existing compute + fullscreen present shaders.
* Ball positions + radii packed into fixed-size GPU buffer (current uniform or migrate early to storage buffer if > ~256 entities is desired).
* Simple flat background & optional debug overlay (ball count, ms/frame).
* Lighting / normals pass retained only if inexpensive; can ship with a “fast flat” mode toggle.

## 4. Physics
* Rapier 2D dynamic bodies for balls; fixed timestep left as variable frame simulation unless instability observed (defer fixed timestep decision until after profiling).
* Continuous Collision Detection OFF initially (perf) – revisit if tunneling visible.
* Gravity: configurable (default mild downward) OR zero + manual impulses. Keep simple.
* All barriers static; no moving geometry in MVP.

## 5. Game State & Flow
Resources:
* `BallCount { current: u32 }`
* `RemainingTargets { current: u32 }`
* `RunPhase { Playing | Won | Lost }`

Systems (Update order sketch):
1. Input (restart key) – early.
2. Physics step (Rapier) – via existing plugin.
3. Collect ball transforms -> GPU buffer packing.
4. Target hit detection (collision events) -> despawn targets -> decrement `RemainingTargets`.
5. Hazard triggers -> despawn ball -> decrement `BallCount`.
6. Win/Lose evaluation (if Playing && RemainingTargets==0 -> Won; if Playing && BallCount==0 -> Lost).
7. UI overlay draw (very small Bevy UI or text bundle). 

## 6. Level Definition (Minimal)
Level = hardcoded spawn list (Rust constant) for MVP:
* Arena bounds (rectangle size).
* Barrier segments (line segments or rectangles).
* Target positions + radius/size.
* Ball spawn positions (Vec). 

Provide at least: `level_test_basic` and `level_perf_stress` (many balls, many targets) for benchmarking.

Future (not now): file-driven RON/JSON, editor tooling.

## 7. Performance Considerations
Primary cost = metaball field evaluation. Immediate tactics:
* Keep ball count moderate (< 256) with current uniform approach. If pressure to exceed: migrate to storage buffer + loop.
* Avoid per-frame allocations in packing; reuse a fixed array or `SmallVec`.
* Provide a compile/runtime switch: `QUALITY=Normal|Flat` where Flat skips normal compute pass.
* Simple frame time logging every N seconds (debug feature gate).

Target metrics (desktop mid-tier GPU):
* Basic level: < 2 ms GPU for metaball pass.
* Stress level (150 balls): < 6 ms GPU (stretch).

## 8. Technical Architecture Mapping
Existing crates:
* `metaball_renderer`: Owns GPU pipelines (compute normals, density, present). Expose API: `MetaballSet { fn clear_and_push(ball: BallData) }` + `update_gpu()` system.
* (Root game crate / demo): Adds physics plugin, spawns entities, registers gameplay systems, integrates renderer system ordering.

Key Data Flow per frame:
`Query<(&Transform, &BallRadius)> -> pack -> write buffer -> run compute shader(s) -> fullscreen present`.

Error Handling: If buffer overflows capacity, log one warning then silently cap additional balls (prevents crash during stress experiments).

## 9. Stretch Items (If Time AFTER Core Loop Works)
Listed in priority order; do not start until MVP criteria met.
1. Basic paddle (kinematic body + player input) to redirect balls.
2. Score system (per target) + simple high score in-memory.
3. Multi-ball spawn event (duplicate existing ball with slight offset).
4. Simple audio stubs (hit / break) using Bevy audio.
5. Configurable level loader (RON file for barriers/targets).

## 10. Explicit OUT-OF-SCOPE (Phase 2+)
* Color-based physics or gating.
* Ball merging / cluster mass logic.
* Portals, switches, moving widgets.
* Boss entities, AI enemies.
* Persistent progression, save system, menus.
* Network or multiplayer.
* Mobile / touch optimization.

## 11. Risks & Mitigations (MVP Only)
| Risk | Impact | Mitigation |
|------|--------|------------|
| Metaball shader too slow with 150 balls | Frame drops | Early profiling; add flat mode; storage buffer migration path. |
| Physics jitter at high velocities | Visual artifacts | Clamp initial impulses; consider fixed timestep if needed. |
| GPU buffer size ceiling (uniform) | Entity cap too low | Abstract packing so backend switch is localized. |
| Scope creep | Delays MVP | Enforce out-of-scope section; PR checklist points to this doc. |

## 12. Minimal UX / Visuals
* Neutral dark background.
* Balls: single color glow (palette index 0).
* Targets: contrasting color; flash white on hit.
* Hazards: red tinted.
* Win / Lose text centered (Bevy TextBundle) + hint: "Press R to restart".

## 13. Test Plan (Lightweight)
Automated (unit-ish):
* Target removal decrements RemainingTargets.
* Ball despawn via hazard decrements BallCount.
* Win condition triggers when last target removed with ≥1 ball.
* Lose condition triggers when last ball removed with ≥1 target.

Manual:
* Performance stress scene (profiling).
* Visual inspection of metaball field with varying counts.

## 14. Implementation Order (Checklist)
1. Basic entity definitions (Ball, Target, Hazard, Barrier components).
2. Spawner for level constant.
3. Physics setup + colliders.
4. Collision event handling (targets & hazards).
5. Game state evaluation system.
6. Renderer integration (pack + update systems ordering).
7. Simple UI overlay.
8. Restart flow.
9. Stress level & perf validation.
10. Stretch (only after validation).

## 15. Future Placeholder (Archive Pointer)
Color mixing, cluster merging/splitting, puzzle gadgets, portals, advanced level types, audio design, and narrative flavor are preserved in prior revision of this file (see git history before commit introducing "MVP Design" heading).

---
MVP Focus Mantra: "Ship the bounce + blob + break loop fast. Everything fancy later."

## 2. Game Objectives and Progression Philosophy

**Core Objectives:** Each level (or “table”) has clear objectives for the player to accomplish before they can progress. The primary objective is often to hit or destroy all the key targets in the playfield – for example, breaking a set of color-coded blocks, activating all switches, or collecting certain pickups. In many levels this echoes the *Arkanoid*-style goal of clearing all breakable blocks; once the field is clear or the mission targets are achieved, the level is completed. Some levels may instead require guiding the ball to a specific goal area (e.g. an exit portal) after activating necessary mechanisms. Scoring is a parallel objective: players rack up points for every successful hit, combo, or trick shot. While scoring high is optional for progression, it feeds into leaderboards, extra lives, or rewards, encouraging replay for improvement.

**Progression Structure:** The game is organized into a series of worlds or chapters, each containing a set of levels with a common theme or new mechanic. For example, the first world introduces basic paddles and one color, the next world adds a new color and related puzzles, later worlds add more complex widgets or hazards. Within a world, levels escalate in difficulty and complexity, teaching the player new skills in a safe setting before testing them in hectic scenarios. To progress to the next world, the player must complete a majority of the levels (e.g. clear 8 out of 10) ensuring they learn core lessons without being strictly stuck on one tricky level. Upon completing all main levels of a world, a final challenge or “boss” level may appear – for instance a level with a unique large target or an enemy orb that requires multiple hits to defeat (similar to how *Arkanoid* featured a boss fight against “DOH” on its final stage). Defeating this unlocks the next world.

**Philosophy on Difficulty:** Progression is designed to be gradual and rewarding. Early stages are forgiving – slower ball speeds, simple layouts – to onboard new players and build confidence. Later stages ramp up challenge with faster action, trickier puzzles, and additional hazards. The philosophy is to keep the player in that optimal challenge zone (the “flow” zone) where the game is neither too easy nor frustratingly hard. Checkpoints or quick retries are provided on longer levels so that failure never feels punishing; the emphasis is on learning by doing. Players are encouraged to replay levels to master them and achieve higher scores or better completion times, but simply finishing a level (even with minimal score) is enough to move on. This dual-layer progression (completion vs. mastery) caters to both casual players and completionists. High skill play is rewarded: for example, achieving an expert score on all levels of a world might unlock a bonus level or other reward, but this is not required to see the main game. Ultimately, the progression aims to introduce one new idea at a time – whether a new color, a new gadget, or a new type of objective – so that the player is continually intrigued and challenged, feeling a sense of discovery as they advance.

## 3. Mechanics Overview

This section outlines the key game mechanics and elements. The game features a rich interplay of physics-based interactions (borrowed from pinball), deliberate aiming (from *Arkanoid*), and unique color-driven behaviors. Below is an overview of each major element:

### Balls (Player Projectiles)

*   **Role:** The balls are the main actors in the game – the objects the player launches and guides to hit targets. The player’s actions (with paddles/flippers) indirectly control the balls’ motion. In many ways, a ball is analogous to a pinball or an *Arkanoid* ball, with hybrid behavior of both.
*   **Physics:** Balls obey realistic 2D physics – they have momentum, bounce off surfaces at appropriate angles, and respond to gravity (generally “downwards” on the playfield). Upon collision with walls, paddles, or objects, they rebound with elastic force. Skilled players can learn to anticipate the ricochet angles much like one does in billiards or *Arkanoid*, allowing planned bank shots.
*   **Metaball Behavior:** Visually and mechanically, balls are represented as metaballs – soft, fluid orbs that can merge together. If two balls touch, they smoothly meld into a single larger blob (see Clusters below for more details). This does not remove any ball; instead, it forms a cluster that carries the combined mass/volume. Because of this, multiple active balls in a level might unite, changing the gameplay dynamic. The metaball nature also means balls can split apart under certain conditions (like hitting a sharp obstacle or by player triggering a split power-up).
*   **Multiple Balls & Multi-ball:** While often the player starts a level with a single ball, there are scenarios with multiple balls. This can happen via power-up (akin to *Arkanoid*’s capsule that creates “several additional balls”) or by design in certain puzzle levels where two differently-colored balls are provided. Multi-ball moments create hectic excitement – suddenly the player is tracking and juggling several projectiles at once, greatly increasing scoring opportunities and chaos. Thanks to the metaball system, multiple balls colliding may merge, so the player can also strategically fuse balls to regain control (merging reduces the count of separate objects to worry about).
*   **Lifecycle:** Typically, a ball is launched into the level from a start point (e.g. a launcher or by releasing it from the paddle). If a ball falls off the bottom of the screen (i.e., past the player’s paddles), it is considered “lost” – analogous to a pinball drain or *Arkanoid* ball loss. The player usually has a limited number of balls or lives per level; losing them all forces a restart. However, there may be ways to earn extra balls (points milestones, special pickups) and certain levels might not have bottomless pits (especially puzzle-oriented ones where the challenge is not to keep the ball alive but to solve a route).

### Paddles and Flippers (Player Controls)

*   **Role:** Paddles (including flippers) are the player’s direct control mechanism for influencing the balls. They are surfaces the player can move or activate to ricochet the balls with force. In design, we include both the classic horizontal paddle (as in *Breakout*/*Arkanoid*) and pinball-style flippers as variants of paddles. These might appear in different levels or even together for different purposes.
*   **Primary Paddle/Flippers:** In most levels, the primary paddles are a pair of flippers located at the bottom left and right of the playfield (imagine a pinball table layout). The player can swing these flippers upward with a button press, launching any ball that hits them back up into the field. The flippers are crucial for preventing balls from dropping off the bottom edge; the player uses them to **keep the ball in play and aim it toward targets**. The flippers function much like in pinball – timing and positioning of the hit will alter the ball’s angle and speed. A skilled player learns to “catch” a ball on a flipper, then shoot it at a chosen angle, providing a sense of mastery over what initially seems like chaotic motion.
*   **Arkanoid-Style Paddle:** In some cases (or perhaps in certain game modes), a single horizontal paddle might be present instead/as well. This paddle slides along the bottom and the player moves it left-right to bounce the ball, exactly as the Vaus ship in *Arkanoid* did. This provides finer lateral control. We might use this in specific puzzle levels where precision outweighs chaos, or as a power-up (e.g., “Extend a flat paddle across the bottom for 10 seconds”). Whether using flippers or a flat paddle, the concept is the same: the player’s tool to send the ball upward.
*   **Additional Paddles:** As the game progresses, we introduce new paddle elements for variety. For example, a level might have an additional flipper on a side wall or halfway up the arena, which the player can also control. These allow new shots (like hitting a ball from the middle of the field). Another example is controllable rotating paddles or gates that the player can toggle to change the ball’s route. All these fall under “paddles” as interactive player-controlled surfaces. They are introduced gradually so the player isn’t overwhelmed. By the later levels, the player might be managing four flippers (two bottom, two mid-level) – leading to high-skill play and impressive trick shots.
*   **Functionality:** All paddles, regardless of type, have a few common functionalities. They bounce the ball with some amplification of speed (the ball comes off faster than it came in if the player hits it actively). They can also impart spin or English to the ball – for instance, hitting the ball with the tip of a flipper can give it a curve. This is subtle but adds depth: an advanced technique is to intentionally spin a ball to alter its bounce trajectory. Paddles might have a limited “charge” ability as well (this is an optional design idea): e.g., holding a flipper engaged for a second could “charge” and smash the ball harder on release, useful for tough targets.
*   **Feedback:** We ensure the paddles feel responsive and powerful. When a ball hits a paddle, the game provides instant audio-visual feedback – a satisfying “thunk” or “boing” sound and a slight screen shake to emphasize a strong hit, reinforcing to the player that they successfully intervened. Controls are tuned so there’s minimal lag; the difference between victory and loss often comes down to split-second paddle flips.

### Clusters (Merged Ball Blobs)

*   **Definition:** A “cluster” refers to two or more balls that have merged into a single blob. Thanks to the metaball visual system, when balls merge, they form one continuous glossy blob, as if two droplets of liquid combined. Clusters behave as one larger ball for movement purposes, but with combined properties of the originals (mass, color, etc.).
*   **Formation:** Clusters form whenever balls of any color touch and merge. For example, if the player releases a multi-ball and the two balls collide, instead of just bouncing off each other, they fuse into one cluster (assuming the game rules allow merge at that moment – merges can be disabled in certain modes if needed to keep balls separate, but generally merging is a core feature). Visually, the merged cluster has a larger size proportional to the total mass. If the balls were different colors, the cluster will take on a blended color (see Color Mechanics below for details on color blending). This smooth merging behavior is a hallmark of metaball physics, creating an organic, fluid feel to the game.
*   **Behavior:** A cluster moves like a single heavier ball. Because it has greater mass, it may be slower to accelerate and not bounce quite as high as a single small ball (the physics can simulate that the combined blob is weightier). This can be advantageous for certain situations – a heavy cluster can plow through breakable obstacles with more force, or stay more stable against a wind/fan hazard – but disadvantageous in others (it might not fit through a narrow corridor or might be harder to fling upward to high places). Clusters also carry the combined momentum of their parts; if two fast-moving balls merge, the resulting cluster conserves that energy.
*   **Splitting:** Clusters can break apart back into separate balls. This can happen by collision with sharp hazards or special splitter gadgets. For instance, a spiked obstacle might pop a cluster into two constituent balls (much like sharp edges split the mercury droplet in *Mercury*). The player might also trigger a deliberate split via a power-up or button when having a cluster is no longer desirable. Splitting reverts the cluster back to smaller components, each with possibly differing colors if the cluster was multi-colored. Importantly, no matter how many times they merge or split, the total number of balls originally in play remains – we’re just dynamically changing their grouping.
*   **Strategic Use:** Managing clusters is a key strategic layer. The player might intentionally merge balls to create a cluster when they want more stability or a specific color combination (for example, merging a red and blue ball to get a purple cluster needed to activate a purple goal). A larger cluster could also make it easier to hit broad targets (bigger collision area) or to apply more force to break through something. Conversely, the player might split a cluster to send balls down different paths simultaneously – e.g., one small red ball goes to trigger a red switch on the left while the remaining blue ball can go to a blue gate on the right. This echoes puzzle designs in games like *Mercury*, where players had to split droplets to multitask and then merge to mix colors. The game will include puzzles that explicitly require splitting one cluster into parts to solve parallel tasks, and then perhaps merging them again for the final objective. Successfully handling clusters gives a satisfying feeling of mastery, as the player is effectively controlling the composition of their projectiles to meet the challenges at hand.

### Widgets and Interactive Elements

*   **Definition:** “Widgets” are interactive objects in a level (other than paddles and balls) that either affect the ball’s motion or state, or constitute part of the puzzle. They are essentially the gadgets and fixtures populating each level’s playfield – drawing from both pinball-style components and new devices unique to the color mechanics.
*   **Bumpers:** A staple from pinball, bumpers are springy obstacles that bounce the ball away with a forceful rebound. When a ball (or cluster) hits a bumper, it gets repelled in a new direction at high speed, often unpredictably. Bumpers usually award points on each hit, encouraging the player to deliberately aim for bumper combos to boost score. They might be placed in clusters to create chaotic ping-pong moments where the ball rapidly ricochets (with fun lights and sounds). While they introduce unpredictability, skilled players can incorporate bumpers into their strategy (e.g., banking a shot off a bumper to reach an otherwise difficult spot).
*   **Switches and Triggers:** Many levels include switches – these could be pressure plates, buttons, or sensors that activate when hit by a ball. Some switches might require a ball of a specific color to activate (see Color Mechanics). Activating a switch can trigger changes in the level: opening a door or gate, toggling a platform, enabling a moving mechanism, etc. For example, hitting a green switch might raise a green platform elsewhere, allowing access to a new section. Often, the level’s puzzle involves hitting switches in the correct order or within time limits. There may be trigger targets that need to all be hit (like a set of 5 targets that light up when struck, similar to pinball drop targets or *Arkanoid*’s brick patterns) – once all are lit, some goal is achieved (e.g., a goal opens or a hazard turns off).
*   **Portals and Teleporters:** To add a spatial puzzle element, some levels have portal widgets. These are paired gates; when the ball enters one, it exits instantly out of the matching portal elsewhere. This allows designing levels where the player must figure out how to use portals to reach secluded areas or to keep the ball in play (for instance, a ball about to be lost at the bottom could fall into a portal that spits it back to the top). Portals can also be color-coded – perhaps only a ball of a certain color can activate a given portal. The visual effect is a satisfying warp that preserves the ball’s momentum through the teleport.
*   **Color Changers:** A crucial widget type is the Color Changer station (sometimes called a Paint Shop in design, inspired by *Mercury*). These are special areas or items that, when a ball passes through, will change the ball’s color to a specific new color. For instance, a red paint widget will turn any ball that touches it red. Color changers are often placed near the start of a section that requires a certain color, serving as the way to prep the ball for the upcoming challenge. They can appear as glowing pools, energy beams, or paint sprayers – visually indicating the color they impart. Some are one-time use, others can be reused as needed. The presence of color changers in a level means the player might have to plan a route: e.g., first go through a blue changer to become blue, use that to get past a blue gate, then later find a red changer to switch color, etc. The color-change mechanic ties directly into puzzle complexity (see Color Mechanics section for more).
*   **Moving Platforms & Obstacles:** There may be dynamic widgets like moving platforms, rotating blades, or swinging gates. These add timing challenges – the player must time the ball’s passage or bounce to get through when the path is clear. For example, a rotating bar with a gap might periodically allow a ball to pass; the player needs to synchronize their shot. Some moving elements could assist the player (e.g., a moving paddle that can catch and redirect the ball along a fixed path) while others are hazards (like a moving blocker that can knock the ball away or into a pit if timed poorly).
*   **Magnetic or Gravity Widgets:** In advanced levels, we introduce elements that influence the physics in interesting ways. Magnetic nodes might attract or repel the ball if it’s of a certain color (for example, a red magnet that pulls red balls but pushes away blue balls). Gravity fields could alter the gravity direction in a region (e.g., a field that when activated makes the ball temporarily fall upward or sideways). These create mind-bending puzzle opportunities where the player has to alter the environment’s physics to guide the ball (an idea inspired by games that manipulate gravity with color, though in our case it would be a localized effect rather than global). Such widgets are used sparingly and clearly indicated, to avoid confusing the player; they serve as the “surprise” mechanics in later stages to keep the game fresh.
*   **Interactive Visual Cues:** All widgets are designed with a minimalist but functional visual style. They use bright, solid colors and simple shapes so the player can instantly recognize their type and state. For example, a switch might be a flat circle that is dark until hit, then it lights up. Portals might be ring shapes with a pulsing glow. Color changers are colored glowing zones. Moving platforms have high-contrast outlines to gauge their position. The simplicity ensures that even amidst fast action, the player can identify interactive elements and understand what they do. Additionally, whenever a widget is triggered (say a door opens), we use small animations or UI arrows to draw attention to the change, so the player isn’t lost about what their action accomplished.

### Goals and Targets

*   **Level Goals:** Each level’s completion condition revolves around specific goal targets. These are the elements the player must hit, collect, or activate to consider the level beaten. Goals come in a few varieties, tailored to the type of level:
*   **Breakable Targets:** Many levels use breakable objects reminiscent of *Arkanoid*’s bricks. These could be blocks, bubbles, or clusters that shatter when hit by the ball. The goal is often to break all of them. Once all such targets are destroyed, the level is cleared (or perhaps a final goal appears). This classic goal gives players a clear checklist and is satisfying as the field gets visibly emptier with each successful hit. Some targets might regenerate or require multiple hits (e.g., a block that cracks on first hit, breaks on second), but the level will communicate remaining durability via visuals.
*   **Switch/Trigger Sets:** Instead of breakables, a level might require activating a set of triggers. For example, light up all five nodes by hitting each at least once. Pinball tables often have such goals (hit all the letters of a word, etc.), and here they serve a similar purpose. When all required switches are lit, an end goal might unlock (like a gate opens revealing the exit). These targets often do not vanish on hit, but change state (off to on). The UI may display progress (e.g., “3/5 switches activated”).
*   **Guiding to Goal Zone:** Some puzzle-oriented levels simply ask the player to guide the ball (or a cluster) to a specific location. This goal zone could be an exit door, a basket, or a highlighted region of the screen. The challenge then is less about hitting everything and more about navigating through the obstacles to reach that zone. It’s analogous to finishing a maze. Often, reaching the goal zone might require first doing other tasks (like collecting keys or unlocking paths). This type is common in levels emphasizing exploration and routing.
*   **Score or Time Goals:** Occasionally, a goal might be meta-gameplay: score X points in Y time or survive for 2 minutes without losing a ball. These are more arcade-like objectives and usually appear in special challenge levels or modes (for example, a bonus level where the goal is purely to rack up points, or an endurance mode separate from the main campaign). While not puzzle-focused, they test the player’s mastery of the mechanics under pressure.
*   **Unique Goal Objects:** Beyond generic blocks or switches, some levels include unique goal objects:
    *   **Clusters of Orbs:** Perhaps a formation of stationary colored orbs that need to be collected or knocked free. Each might add to the player’s cluster when hit (imagine hitting a blue orb causes it to merge into your ball, effectively “collecting” it). The goal could be to gather all orbs in the level into your cluster, achieving a full collection.
    *   **Enemy Characters:** Though the game isn’t heavily character-based, now and then we might introduce an enemy “boss” or entity that functions as a target. For instance, a blob-like enemy that roams the screen and must be hit multiple times to be defeated (each hit changes its color or size to indicate progress). This can serve as a boss battle at the end of a world – requiring technique to hit a moving target and possibly specific colors to damage it.
*   **Progress and Feedback:** The game provides clear feedback on remaining goals. For instance, a counter of how many targets remain, or visual indicators (unbroken blocks vs broken). There is often a final flourish when the last goal is achieved: the game might slow down or zoom out slightly, showing a celebratory effect (particles, sound fanfare), and then either end the level or prompt the player to send the ball into an exit to officially finish. This closure ensures players recognize they’ve met the objective. It’s analogous to *Arkanoid* advancing to the next stage after the last brick is gone – we’ll have a distinct “level clear” confirmation.

### Hazards and Fail States

*   **Bottomless Pit / Draining:** The most constant hazard is the bottom edge of the playfield (in standard levels) – letting the ball slip past the paddles means losing that ball. Just as in pinball and *Arkanoid*, the player must always guard against the ball draining out. If it does, one life or ball is lost; the level may restart or the player continues with one fewer ball, depending on mode. This looming hazard creates tension and encourages skillful play with the paddles.
*   **Spikes and Shredders:** Physical hazards like spikes, blades, or crushers will harm the ball on contact. Since our “balls” are blobs, getting spiked might cause the ball to split into pieces (if a cluster, it breaks apart; if a single ball, it might lose a chunk of mass or break into two smaller balls). In some cases, a spike could outright destroy a small ball, effectively costing a life. These hazards often are placed in challenging spots, like guarding a high-value target or lining a narrow passage the player must traverse carefully. They introduce risk-reward decisions: Do I attempt a tricky shot between the spikes to hit that target, or deal with it another way?
*   **Hazardous Surfaces:** Some surfaces could be sticky goo, sand, or water that dampens the ball’s momentum. While not killing the ball, they act as hazards by making shots less effective. For example, a patch of goo on a wall might stop a ball dead in its tracks, potentially causing it to drop straight down (toward that dreaded bottom pit). Or a sand trap might slow the ball so much that the player has to hit it out with extra force. These are environmental hazards that make navigation more difficult.
*   **Enemy Hazards:** In advanced or boss levels, there might be moving enemy objects – e.g., a drone or critter that patrols. If the ball hits the enemy in the wrong way, it could grab or deflect the ball unpredictably (or even “eat” it, causing a loss). The player might need to disable the enemy first (perhaps by hitting it from behind or using a certain color) to neutralize the hazard.
*   **Color-specific Hazards:** Some dangers interact with the color system. For instance, a laser barrier that destroys any ball of the wrong color (only a ball of matching color can pass safely). Or a drain field that slowly saps the color/energy from a ball – if a ball stays too long in that field, it might “fade out” (simulating losing mass or life). This was inspired by the idea in *Mercury* that dropping below a certain mass fails the level, though here we apply it via color/energy. The player might see their red ball turning dull if it’s in a blue hazard zone, warning them to get out quickly. Such hazards enforce puzzle constraints (e.g., you must be a blue ball to go through the blue fire, any other color will be destroyed).
*   **Time Pressure:** While not a physical hazard, timers on some levels act as a fail condition – if the player doesn’t complete objectives in time, it’s game over for that attempt. An example is a level that simulates a self-destruct sequence: you have 60 seconds to hit all the targets. Time-based pressure can be very engaging in moderation, pushing players into a flow state of intense focus. We will clearly mark timed levels and likely keep them as optional challenges or separate mode (so players who prefer relaxed puzzle solving can enjoy the main campaign without strict timers).
*   **Hazard Feedback and Fairness:** The design philosophy with hazards is to make them visible and understandable. We use bright warning colors (often red or yellow) and distinct shapes for hazards (spikes look spiky, lasers glow, etc.). The first time a new hazard type appears, the level gives the player a relatively safe space to see it in action (perhaps an early part where a ball can touch a hazard in a low-stakes way) so they learn the rule. If a hazard causes a loss, the game provides feedback – e.g., the ball pops with an animation and a sound, perhaps a message like “Ball lost!” or an icon of that hazard so the player knows what got them. This way, failures become lessons and not mysteries. Generous checkpoints or quick restarts make sure the player can try again right away, adjusting their strategy.

## 4. Color-Based Mechanics and Physics

Color is a defining feature of *Color Fusion*. It’s not just an aesthetic detail, but woven deeply into gameplay mechanics and even physics behaviors. This section explains how color works in the game:

### Color States and Properties

*   **Colored Balls:** Every ball (or cluster) has a color, which can be one of the primary colors (Red, Green, Blue in our system) or a composite (like Yellow, Cyan, Magenta, etc., resulting from mixing). The color is immediately visible as the fill color of the metaball. A newly launched ball might start as a neutral gray or a default color depending on level design. From there, it can change.
*   **Color-Based Physics:** Each color can be associated with slight differences in physical behavior – this is the “color-based physics system” driving gameplay. For example, we might assign: Red balls are denser/heavier (they don’t bounce as high but can smash through breakable blocks more effectively), Blue balls are bouncy and light (they bounce higher and travel faster but are easily deflected by wind or currents), and Green balls could be medium weight but very sticky (tending to cling to surfaces or slow down on impact). These are illustrative; the exact properties will be tuned for balance. The idea is that choosing the right color for a situation alters the physics in your favor. This concept has parallels to games like *Colour Bind*, where different colors experienced different gravity strengths – in our game the differences will be more subtle but noticeable to skilled players.
*   **Visual Indication:** The game provides clear feedback on these properties. For instance, a red ball might have a slight “aura” or particle effect suggesting mass (little sparks when it hits, implying force), while a blue ball leaves a faint trail indicating its lightness. These effects are kept minimal (so as not to clutter the screen) but help players intuitively feel the difference. Through experimentation, a player might observe “My red ball isn’t knocked aside by the fan widget as much as the blue one was – red must be heavier.”

### Merging and Color Mixing

*   **Merge Mechanics:** When two differently colored balls merge into a cluster, their colors combine following an additive mixing model (since these are glowing energy balls, we use light color addition). This is similar to how *Archer Maclean’s Mercury* handled color mixing based on RGB. For example, merging a red and a green ball yields a yellow cluster; blue + red yields magenta, green + blue yields cyan. If all three primaries merged, you’d get a near-white multicolored blob. The resultant color matters for gameplay because it might satisfy certain color-specific requirements. A yellow cluster would be recognized as both red and green for triggers (or we design the rules such that some puzzles specifically need a secondary color like yellow).
*   **Partial Mixing and Layering:** We also consider an alternative rule for complexity: if balls merge but one is much larger than the other, the color might tint towards the larger one’s color. However, to keep it straightforward, generally any merge will produce a uniform blended color (the exact shade can be an average weighted by size). The aim is that players can predict the outcome: “I need purple to open that gate, so if I merge my blue ball with a red ball, I’ll get purple.” The game might include a simple color-mixing chart in a tutorial or on the HUD for reference.
*   **Splitting and Color Retention:** When a cluster splits, how do colors distribute? We design it such that color splits logically: if the cluster was a uniform color (say pure yellow), when breaking into two pieces, one piece might be pure red and the other pure green (essentially returning to the original components that formed yellow). This is if the split happens along the lines of the original merge. If a cluster splits by force randomly, we might define it as splitting into two of the cluster’s same color (like a cell division) – but that could let players duplicate colors, which might break puzzles. So more likely, splits will occur via designated splitter gadgets that are calibrated to break off a specific color portion. For simplicity in most puzzles, any required split of colors will be set up clearly (e.g., you merge to get purple, use it, then a splitter might break the purple into red and blue again if needed later).
*   **Color Persistence:** Balls retain their color until changed by a mechanic (merging, passing through a color changer, or certain hazard effects). There’s no passive time-based color fading (aside from hazard fields deliberately causing it). This ensures that when you obtain a certain color, you can count on keeping it while you execute your plan, unless you take it somewhere that alters it.

### Color-Based Interactions

*   **Color Gates and Doors:** A common use of color is gated access. A gate might only let a ball of a certain color pass or trigger. For example, a Blue Barrier will physically block any ball that is not blue, acting like a wall. But the moment a blue ball touches it, it might vanish or open (and potentially stays open afterward, or closes again after passage depending on design). Similarly, a Red Door Switch might require a red ball to hit it to toggle a door. This creates key-and-lock style puzzles: the player sees a barrier with a color icon, and knows they need to bring that color to this point.
*   **Color Triggers and Events:** Beyond doors, events can be tied to color triggers. For instance, a puzzle where the player must cause a chemical reaction by bringing a green ball into a reactor, which then triggers something because green represents, say, activated state. We could have color-coded explosive blocks that only explode when hit by a ball of a matching color (like red bomb blocks triggered by red balls, yielding a strategic use: if you send a red ball, you can clear a path via explosion, but other colors won’t trigger it).
*   **Combining Colors for Effects:** Some complex interactions might require combining colors in the environment. Imagine three sockets in the level colored R, G, B. If the player can manage to have three balls (or split one cluster into three) and have each in one of those sockets simultaneously, that could open a secret or complete an objective – essentially recreating white light or some complete set. Or consider a prism hazard that splits any cluster into components: you might roll a multi-colored cluster into it, and it splits into separate colored balls which then shoot out along different paths (a dramatic but purposeful event perhaps used in a late-game level).
*   **Negative Color Interactions:** Hazards might also reference color: e.g., a Nullifier Field that strips color from any ball passing through, turning it back to neutral (which might be required if you accidentally became the wrong color and need to reset). Another hazard could be a Wrong Color Trap – like a red flame that intensifies if a non-red ball touches it, making you quickly lose control. We use these sparingly so as not to frustrate, primarily to enforce that certain areas truly demand the correct color.
*   **Feedback and Clarity:** It is critical that players understand the color requirements. All color-interactive elements (gates, switches, etc.) are clearly marked with colored highlights or symbols. If a ball hits something and nothing happens because it’s the wrong color, the game provides a subtle cue: the object might flash the color it needed or simply not react but perhaps show a “red key icon” briefly. Conversely, if a correct color interaction happens, we reward it with a distinct sound and visual flourish (like a colored spark or the gate dissolving in a glow). We want players to connect the dots: “Ah, my blue ball opened that blue gate – got it.” Over time, they’ll plan ahead for these: “I see a green switch up there; I likely need to turn my ball green first.”

### Example Scenarios

To illustrate the above mechanics, here are a couple of hypothetical in-game scenarios:

*   **Puzzle Example:** The level has a lower section sealed by a green door and an upper section containing a green paint power-up. The player starts with a blue ball. They bounce around to reach the paint, turning their ball green. Now green, they return and pass through the green door. Inside, there are two switches: one requires a red ball, one requires a blue ball – but the player is currently green. Conveniently, a splitter hazard here cuts the green ball into its components: a red ball and a blue ball. Now they hit the red switch with the red ball and the blue switch with the blue ball (possibly by controlling them one at a time or simultaneously with careful banking). That opens the exit portal. The player then guides the ball(s) into the exit, level clear. This scenario forced color changing, splitting, and multi-ball skill all in one coherent puzzle.
*   **Action Example:** A level might have multiple colored bumpers that give extra points if hit with the matching color ball (a red bumper lights up when a red ball hits it, granting a score bonus). The player can try to cycle their ball through color changers mid-play to “score the rainbow”. For instance, they launch the ball (starting white). It bounces off some normal bumpers, then goes through a red changer – now it’s red. The player aims for the red bumper zone to rack up points. Then it hits a blue changer – now blue – and they try to hit the blue bumpers, and so on. This isn’t required to beat the level, but it provides a layer of challenge for score-chasers, utilizing color in a fast-paced context.

In summary, color in *Color Fusion* adds a puzzle layer on top of the physics. It requires the player to not only think about where to send the ball, but in what state. Merging and splitting blobs to get the right hues at the right time creates engaging “aha!” moments, while the immediate effects of color (like how it bounces or what it triggers) keep the moment-to-moment gameplay fresh and varied.

## 5. Types of Levels and Unique Level Designs

The game features a variety of level types, each emphasizing different aspects of the gameplay. This diversity ensures the experience stays fresh and caters to different player strengths (reflexes, strategy, etc.). Below are the primary types of levels and what makes each unique:

1.  **Classic Breaker Levels (Arcade Action):** These levels resemble a traditional breakout game. The playfield is filled with breakable blocks or targets, and the objective is simply to destroy them all. They tend to be fast-paced and action-heavy. The addition of color means some blocks might only be breakable by certain colored balls, adding a twist not found in classic *Arkanoid*.
2.  **Puzzle Adventure Levels:** These stages are more about exploration and problem-solving. The screen may scroll or be larger than one viewport, containing multiple rooms or sections connected by gates, teleporters, etc. The goal is often to reach an exit or trigger a sequence of switches, rather than clear all objects. These levels highlight the color mechanics: the player will encounter color-coded gates, need to merge/split balls, and use widgets in clever ways.
3.  **Multi-Ball Mayhem Levels:** A special set of levels focused on multi-ball gameplay. Here, the player is given multiple balls at once from the start. The design might be symmetrical or open-area to allow multiple balls to bounce freely. The challenge is in not becoming overwhelmed; players must rapidly prioritize which ball to hit.
4.  **Time Attack Levels:** These are levels where the pressure is on the clock. The layout might be simpler, but the player has a strict time limit to either score a quota of points or hit a set of targets. They are great for players who love arcade thrill and competition.
5.  **Boss and Challenge Levels:** At the end of a world, the player faces a boss level. A boss level might feature a central boss target – for example, a large multi-color blob enemy that moves and fights back. The player might have to hit it with specific colors in sequence to defeat it.
6.  **Hybrid Levels:** Many levels will blend elements of the above types. For instance, a level could start as a puzzle and end in an action flurry. This hybrid approach keeps players on their toes.

**Level Theme and Aesthetic:** Each world of levels has a distinct theme that reflects in background art, colors used, and widget styles. For example:

*   **World 1: Digital Neon Grid** – A TRON-like virtual arena with simple geometry.
*   **World 2: Chromatic Jungle** – Lush forest tones and natural bumpers (vines, etc.).
*   **World 3: Crystal Cavern** – Lots of reflective surfaces and glass blocks that split light.
*   **World 4: Mechanical Factory** – Conveyer belts, magnets, heavy machinery.
*   **World 5: Cosmic Arcade** – A mashup of pinball arcade elements in space.

## 6. Controls and Interaction Methods

The control scheme is designed to be simple to learn but with enough nuance to allow skill development.

*   **Basic Controls (Keyboard/Gamepad):**
    *   **Left/Right Flipper:** Two keys (e.g., Left/Right Arrow or A/D) control the flippers. On a gamepad, triggers or shoulder buttons are used.
    *   **Move Paddle:** If a sliding paddle is present, arrow keys or an analog stick control its lateral movement.
    *   **Launch Ball:** A dedicated key (e.g., Spacebar) launches a new ball, possibly with a charge-and-release mechanic.
    *   **Nudge/Tilt:** An advanced control to give the playfield a small push. Overuse results in a temporary “Tilt” penalty, disabling paddles.
*   **Special Actions:** An extra button may be used for power-ups, such as a manual split/merge toggle.
*   **Mouse Controls (Optional):** Mouse movement could control a sliding paddle, with clicks for flippers.
*   **Touch Controls (Mobile/Tablet):** Virtual button regions on the screen for flippers, and drag-to-move for paddles.
*   **Accessibility and Customization:** Key rebinding, sensitivity adjustments, and optional assists like slow-motion will be available.
*   **Feedback:** Controller vibration and screen shake will provide feedback for impacts and events.

## 7. Desired Player Emotions and Engagement Styles

We want *Color Fusion* to resonate emotionally with players and support various styles of engagement.

*   **Flow and Immersion:** By balancing challenge and skill, we aim to induce a flow state where players become fully absorbed in the action.
*   **Competence and Mastery:** A core goal is to foster a sense of mastery. As players learn the physics and color mechanics, they will gain confidence and feel proud of their skill growth.
*   **Curiosity and Exploration:** The game will spark curiosity with new mechanics and hidden secrets, rewarding players who experiment and explore.
*   **Challenge and Tension:** We will create moments of nail-biting tension, especially when a player is close to completing a difficult objective. Overcoming these challenges leads to exhilaration.
*   **Relaxation and Playfulness:** Not all levels will be high-stress. Some will have a more sandbox feel, allowing players to relax and enjoy the physics playground.
*   **Connection and Triumph:** Finishing a level, and especially a world, should feel like a significant victory, reinforced by celebratory visuals and sounds.

## 8. Art and Visual Direction (Functional and Minimalist)

The visual design is minimalist, focusing on clarity, functionality, and a sleek, modern style.

*   **Metaball Aesthetic:** Balls are rendered as shiny, pseudo-2.5D orbs that merge smoothly into blobby shapes, with soft glows and seamless color blending.
*   **Color Palette:** The palette is bold and limited, using bright, solid colors for interactive elements against contrasting neutral backgrounds to ensure gameplay elements stand out.
*   **Functional Simplicity:** Every visual element is designed with its function in mind, using intuitive shapes and clear outlines.
*   **Neon/Tron Influence:** The overall vibe is inspired by futuristic neon designs, with glowing lines and grid patterns, but with a modern, minimalist polish.
*   **Pseudo-2.5D Elements:** Subtle 3D-esque touches like drop shadows and parallax motion give the game a premium look without sacrificing 2D clarity.
*   **Minimal Character/Story Visuals:** The game is not narrative-heavy. Any story is conveyed through abstract visuals or brief text.
*   **Performance:** Visuals are optimized for fluid, high-frame-rate performance.
*   **Consistent Design Language:** A clear visual language helps players quickly parse levels (e.g., hazards are always spiky and warm-colored).
*   **UI and HUD:** The heads-up display is minimal, with clean fonts and simple icons kept to the edges of the screen.
*   **Colorblind Accessibility:** To ensure accessibility, color-coded elements will also have unique shapes or symbols. Palettes will be tested for common colorblindness issues.

## 9. Audio Design Themes

Audio is crucial for feedback and atmosphere, aligned with the game’s modern, minimalist style.

*   **Sound Effects (SFX):** Crisp, satisfying sounds will reinforce every action:
    *   **Ball Hits:** Sharp "knocks" for paddles, softer "pings" for walls, and upbeat "boings" for bumpers.
    *   **Events:** Glassy tinkles for breaking targets, electronic beeps for switches, and whooshes for portals.
    *   **Metaballs:** A gooey "slurp" for merging and a "pop" for splitting.
    *   **Color Changes:** A short, rewarding tone or chord.
    *   **Hazards:** Alerting sounds like a sharp "crunch" or "zap," and a descending whine for losing a ball.
*   **Music:** The soundtrack will be electronic (synthwave, chillstep) and dynamic.
    *   **Style:** Melodic but not distracting, with a cohesive theme across the game.
    *   **Dynamic Music:** The music will adapt to gameplay, becoming more intense during action sequences (e.g., multi-ball) and calmer during puzzle-solving moments.
    *   **Variation:** Each world will have its own track to reinforce the theme.
*   **Mix:** Important SFX will cut through the music. Stereo panning will be used for positional awareness. Volume sliders for music and SFX will be included.

## 10. Expansion Potential (Future Features and Modes)

*Color Fusion* is designed for future growth.

*   **Level Editor & Community Content:** A user-friendly level editor would allow players to create and share their own levels, fostering a community and providing endless content.
*   **New Worlds and Mechanics:** Post-launch expansions could add new worlds, each introducing a fresh mechanic like gravity flips or levels played in darkness.
*   **Challenge/Remix Mode:** Hardcore modes for veteran players, such as time trials, modified physics, or a randomized endless arcade mode.
*   **Multiplayer Modes:**
    *   **Cooperative Play:** Two players working together to clear levels.
    *   **Competitive Play:** A versus mode where players compete for score or send hazards to each other's playfield.
*   **Online Leaderboards:** Global leaderboards for high scores and times to motivate competitive play.
*   **Additional Power-ups and Widgets:** New power-ups (e.g., shape-shifting balls) and widgets (e.g., moving enemies) could be added in updates.
*   **Thematic Expansions:** New story scenarios or visual themes to refresh the experience.
*   **Crossovers and Events:** As a fun idea, the game could have crossover content from other indie games or classic games (subject to partnerships). For instance, a special level that is a nod to classic Arkanoid – a recreation of an Arkanoid stage but with our physics and color twists, possibly done as a free celebratory update (tying into how Arkanoid has legacy). Or seasonal events: a Halloween level set (with spooky color effects) or holiday event where a level is shaped like a Christmas tree with lights as targets, just for festive engagement.
