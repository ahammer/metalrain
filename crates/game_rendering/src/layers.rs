use bevy::prelude::*;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Component)]
pub enum RenderLayer {
    Background = 0,
    GameWorld = 1,
    Metaballs = 2,
    Effects = 3,
    Ui = 4,
}

impl RenderLayer {
    pub const ALL: [RenderLayer; 5] = [
        RenderLayer::Background,
        RenderLayer::GameWorld,
        RenderLayer::Metaballs,
        RenderLayer::Effects,
        RenderLayer::Ui,
    ];

    pub fn order(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            RenderLayer::Background => "Background",
            RenderLayer::GameWorld => "GameWorld",
            RenderLayer::Metaballs => "Metaballs",
            RenderLayer::Effects => "Effects",
            RenderLayer::Ui => "Ui",
        }
    }
}

impl Display for RenderLayer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
}

#[derive(Debug, Clone, Reflect)]
pub struct LayerConfig {
    pub layer: RenderLayer,
    pub blend_mode: BlendMode,
    pub enabled: bool,
    pub clear_color: Color,
}

impl LayerConfig {
    pub fn new(layer: RenderLayer, blend_mode: BlendMode, clear_color: Color) -> Self {
        Self {
            layer,
            blend_mode,
            enabled: true,
            clear_color,
        }
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct LayerToggleState {
    pub configs: Vec<LayerConfig>,
}

impl Default for LayerToggleState {
    fn default() -> Self {
        let configs = vec![
            LayerConfig::new(RenderLayer::Background, BlendMode::Normal, Color::BLACK),
            LayerConfig::new(RenderLayer::GameWorld, BlendMode::Normal, Color::NONE),
            LayerConfig::new(RenderLayer::Metaballs, BlendMode::Additive, Color::NONE),
            LayerConfig::new(RenderLayer::Effects, BlendMode::Additive, Color::NONE),
            LayerConfig::new(RenderLayer::Ui, BlendMode::Normal, Color::NONE),
        ];
        Self { configs }
    }
}

impl LayerToggleState {
    pub fn config(&self, layer: RenderLayer) -> Option<&LayerConfig> {
        self.configs.iter().find(|cfg| cfg.layer == layer)
    }

    pub fn config_mut(&mut self, layer: RenderLayer) -> Option<&mut LayerConfig> {
        self.configs.iter_mut().find(|cfg| cfg.layer == layer)
    }
}
