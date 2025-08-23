// Phase 8 (scaffold): Debug tools crate placeholder.
// Provides DebugToolsPlugin; real overlay / stats systems added later and will be cfg(feature = "debug") at workspace level.

use bevy::prelude::*;

pub struct DebugToolsPlugin;

impl Plugin for DebugToolsPlugin {
    fn build(&self, _app: &mut App) {
        // Future additions:
        // - Overlay UI
        // - Stats collection
        // - Mode cycling
        // - Optional metaballs debug (gated by feature "metaballs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_adds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DebugToolsPlugin);
    }
}
