# Physics Playground

**An Interactive Physics Sandbox & Integration Showcase**

> *The Physics Playground is the ultimate testbed and demonstration environment where all game systems come together in perfect harmony. It's designed to showcase the full power of the modular architecture while remaining delightfully simple to use and understand.*

---

## Purpose

The Physics Playground serves three critical roles:

1. **Integration Laboratory** – A living proof that all crates (`game_core`, `game_physics`, `metaball_renderer`, `game_rendering`, `event_core`, `widget_renderer`) work together seamlessly
2. **User Experience Prototype** – A hands-on environment to experiment with physics parameters, visual effects, and gameplay interactions
3. **Living Design Reference** – Documentation through demonstration, showing the ideal way these systems should integrate

This is intentionally a **minimal-code showcase** – almost everything happening here is provided by the shared crate infrastructure, making it an excellent reference for how to compose a complete game experience from modular components.

---

## What You Can Do

### Interactive Physics Experimentation

**Spawn and Observe**

- Click anywhere in the play area to spawn balls with randomized velocities
- Watch as balls interact through realistic 2D physics simulation
- See balls attract to each other through configurable clustering forces
- Observe velocity constraints keeping motion visually readable

**Real-Time Tuning**

- Adjust gravity strength and direction via intuitive sliders
- Fine-tune ball bounciness (restitution) from perfectly elastic to dampened
- Control surface friction for realistic rolling behavior
- Configure clustering attraction strength and effective radius
- Set minimum and maximum speed limits to prevent chaos or stagnation
- See changes take effect immediately – no restart required

**Visual Feedback**

- Yellow velocity vectors show each ball's current motion
- Smooth metaball rendering creates organic, blob-like visual appeal
- Color-coded balls indicate different clusters or states
- Real-time performance metrics display frame rates and entity counts

### Gameplay Mechanics Testing

**Core Elements**

- Spawn static walls that balls bounce off
- Create target zones that react to ball collisions
- Place hazards that affect nearby balls
- Control paddles (kinematic bodies) that redirect ball paths
- Define spawn points with visual indicators of active/inactive states

**Event-Driven Interactions**

- Reset the entire simulation with a single keystroke
- Pause and resume physics for detailed observation
- Trigger gameplay events (spawning, destruction, state changes) via input
- Watch as the event system processes inputs deterministically

**Level Design Validation**

- Test arena configurations before committing to gameplay code
- Verify collision boundaries and interaction zones
- Prototype spawn patterns and timing
- Validate win/lose conditions with actual physics behavior

---

## User Stories

### "I want to understand how the physics feels"

Launch the playground, spawn a few dozen balls, and experiment with the sliders. Increase gravity to simulate falling game mechanics, or reduce it for floaty space-themed gameplay. Adjust restitution to feel the difference between bouncy rubber balls and heavy metal spheres.

### "I need to verify all systems work together"

Run the playground as your integration test. Every crate is active: physics simulating, metaballs rendering, widgets visualizing, events processing, the compositor layering everything correctly. If it runs smoothly here, your architecture is solid.

### "I'm designing a new game mode and need to prototype"

Use the playground as your canvas. Spawn walls to create boundaries, place targets where you want objectives, add hazards for difficulty. See immediately how players would interact with your design. Iterate rapidly without writing code.

### "I need to troubleshoot a physics issue"

The playground exposes all the knobs. If clustering seems too aggressive, lower the strength. If balls escape the play area, check your velocity clamps and boundary colliders. The visual velocity vectors make debugging motion problems trivial.

### "I want to stress-test rendering performance"

Spawn hundreds of balls and watch the frame rate. The metaball renderer's GPU compute pipeline should handle large counts efficiently. Use the diagnostics panel to identify bottlenecks between physics simulation, rendering, and event processing.

---

## Key Features

### Zero-Code Philosophy

Almost no logic lives in the playground itself – it's purely composition and configuration. This demonstrates the power of the modular architecture: complex behaviors emerge from combining simple, well-designed systems.

### Complete System Integration

- **game_core**: Provides all gameplay entities (balls, walls, targets, hazards, paddles, spawn points)
- **game_physics**: Handles collision detection, gravity, clustering forces, velocity management
- **metaball_renderer**: GPU-accelerated blob rendering with field computation and normal mapping
- **game_rendering**: Multi-layer compositor ensuring UI, gameplay, and effects render in correct order
- **event_core**: Deterministic input processing, middleware chains, event handlers
- **widget_renderer**: Visual representation of non-ball entities with animations and highlights

### Real-Time Configuration

Every physics parameter is adjustable at runtime through an intuitive UI. No configuration files to edit, no restart required. Tweaking is immediate and satisfying.

### Visual Debugging

- Velocity gizmos show motion direction and magnitude
- Colored overlays indicate entity states
- Layer visualization ensures rendering order is correct
- Performance metrics provide instant feedback

### Deterministic Behavior

Thanks to the event_core architecture, identical inputs produce identical results. This makes the playground perfect for:

- Recording and replaying experiments
- Regression testing physics changes
- Comparing behavior across code versions
- Debugging race conditions (there aren't any!)

---

## Design Ideals

### Demonstration Quality

Every aspect should feel polished and responsive. This isn't a throwaway test – it's the reference implementation others will study and emulate.

### Clarity Through Simplicity

Anyone looking at the playground code should immediately understand:

- Which crates are involved
- How they connect together
- What each one contributes
- Why the architecture makes sense

### Maximum Leverage

The playground achieves rich functionality with minimal custom code by:

- Using plugins instead of writing systems
- Configuring resources instead of implementing logic
- Composing bundles instead of spawning raw entities
- Leveraging event handlers instead of adding conditional branches

### Extensibility Preview

While the playground itself stays minimal, it should be obvious how to extend it:

- Add new middleware to the event chain
- Register additional handlers for custom interactions
- Spawn new entity types from game_core
- Tune rendering layers for visual effects
- Integrate additional physics forces

---

## Controls

### Mouse

- **Left Click**: Spawn a ball at cursor position with randomized velocity

### Keyboard

- **R**: Reset entire simulation to initial state
- **P**: Pause/resume physics simulation
- **Space**: Primary action (triggers event system – behavior depends on active handlers)
- **WASD / Arrow Keys**: Movement inputs (can be mapped to paddle control)

### UI Panel

- **Gravity Sliders**: Adjust X and Y components of gravitational force
- **Restitution**: Control ball bounciness (0.0 = no bounce, 1.0 = perfect elastic)
- **Friction**: Surface resistance (0.0 = frictionless, 1.0 = maximum drag)
- **Clustering Strength**: Attraction force between nearby balls
- **Clustering Radius**: Effective distance for attraction forces
- **Max Ball Speed**: Upper velocity limit to prevent runaway acceleration
- **Min Ball Speed**: Lower velocity limit to keep things moving

---

## Technical Integration

### Plugin Architecture

The playground adds minimal custom systems – nearly everything comes from:

```
GameCorePlugin          → Entities, events, resources
GamePhysicsPlugin       → Rapier integration, forces, constraints
MetaballRendererPlugin  → GPU compute, field textures, coordinate mapping
GameRenderingPlugin     → Compositor, layers, camera management
EventCorePlugin         → Input processing, deterministic event queue
WidgetRendererPlugin    → Visual meshes, animations, highlights
```

### Resource Configuration

Exposes critical resources for UI manipulation:

- `PhysicsConfig`: All physics parameters
- `GameState`: Pause, reset, win/lose states
- `ArenaConfig`: Boundary definitions
- `MetaballRenderSettings`: Visual quality, world bounds
- `EventQueue`: Access to event journal for debugging

### Event Flow

```
User Input → KeyMappingMiddleware → GameEvent
          → DebounceMiddleware → CooldownMiddleware
          → EventQueue (frame-atomic)
          → Reducer (PostUpdate)
          → Handlers (spawn, destroy, state change)
          → ECS World mutation
```

### Coordinate Spaces

- **World Space**: Physics simulation, game logic (arbitrary units)
- **Metaball Texture**: GPU compute target (pixel coordinates)
- **Screen Space**: User interaction, cursor positioning
- **UV Space**: Shader sampling, compositor blending

The `MetaballCoordinateMapper` handles all transformations automatically.

---

## Success Criteria

The Physics Playground succeeds when:

1. **It's Delightful** – Users immediately understand it and want to play with parameters
2. **It's Instructive** – Developers can read the code and understand the architecture instantly
3. **It's Reliable** – No crashes, no frame hitches, deterministic behavior every time
4. **It's Complete** – Every crate is utilized, every feature is showcased
5. **It's Minimal** – No unnecessary code, no redundant systems, pure composition

---

## Rebuild Philosophy

When the time comes to rebuild the playground from scratch:

1. **Start with Dependencies** – Add crate references in Cargo.toml
2. **Add Plugins in Order** – Core → Physics → Rendering → Events → Widgets
3. **Configure Resources** – Set up initial state and tuning ranges
4. **Wire UI Bindings** – Connect sliders to resource fields
5. **Add Input Handlers** – Map keys to event emissions
6. **Test Each Layer** – Verify one system at a time

The rebuild should take hours, not days, because all the hard work lives in the crates. The playground is just the conductor bringing the orchestra together.

---

## Future Enhancements

### Planned Features

- **Preset Scenarios**: One-click arena configurations (empty, maze, pinball, orbital)
- **Recording & Playback**: Save input sequences, replay deterministically
- **Comparison Mode**: Side-by-side physics parameter testing
- **Export Configuration**: Save tuned parameters to files for use in actual games
- **Visual Regression**: Automated screenshot comparison for rendering changes
- **Performance Profiling**: Detailed frame-time breakdown by system

### Possible Extensions

- **Level Editor Mode**: Click-and-drag placement of walls, targets, hazards
- **Custom Event Injection**: UI for manually triggering specific event types
- **Advanced Diagnostics**: Real-time graphs of velocity distribution, clustering density
- **Multi-Preset A/B Testing**: Toggle between configurations mid-simulation
- **Camera Controls**: Pan, zoom, follow specific balls
- **Shader Live Reload**: Hot-swap compute shaders without restart

---

## Why This Matters

The Physics Playground is more than a demo – it's a **design philosophy made tangible**. It proves that:

- **Modularity Works**: Complex systems can be simple when properly decomposed
- **Composition Scales**: Small pieces combine into rich experiences
- **Testing Needn't Be Boring**: Validation can be interactive and enjoyable
- **Documentation Lives**: The best spec is a working reference implementation

When someone asks "How do I use these crates together?", the answer is simple: "Look at the Physics Playground."

---

*The Physics Playground: Where Theory Becomes Tangible*
