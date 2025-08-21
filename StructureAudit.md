I'm noticing the good
- Plugin Style Architecture
and the bad
- Monolithic


Lets focus on the bad. We have one src dir, it's flat and we keep adding plugins.

I'm going to propose the following multi-crate solution.

crates/core             // Base Crate with Shared Interfaces
crates/rendering/...    // Rendering related Crates (I.e. Shaders and Rendering helpers, Backgrounds, etc)
crates/physics/...      // Physics related Crates
crates/ui/...           // UI Related Crates
crates/state/...        // Game State related class
crates/debug/...        // Debug Utilities
game/               // Entry Point to Game (Minimal)
debugIntegration/   // Entry Point to Game w/debug tools (Minimal)
harnesses/...       // Samples for specific other crates, i.e. Metaballs, or Ball Physics. So things can be decomposed and tested in isolation.

This is only an example/general direction, please review the idea of this, and then propose based on the existing code and structures

My general preference for conversion/migrating follows this.

1) Isolate component (Define the area to encapsulate)
2) Define interfaces in modules
3) Create and Test Replacement
4) Delete old code and migrate to drop in replacement

We should use test harnesses extensively to validate and integrate crates at-will for useful and efficient test-harnesses
