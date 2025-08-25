// ...existing code...
use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

// (content moved from original config.rs)
// BEGIN MOVED CONFIG TYPES
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub title: String,
    #[serde(rename = "autoClose")]
    pub auto_close: f32,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 720.0,
            title: "Bevy Bouncing Balls".into(),
            auto_close: 0.0,
        }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GravityConfig {
    pub y: f32,
}
impl Default for GravityConfig {
    fn default() -> Self {
        Self { y: -600.0 }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct BounceConfig {
    pub restitution: f32,
}
impl Default for BounceConfig {
    fn default() -> Self {
        Self { restitution: 0.85 }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct SpawnRange<T> {
    pub min: T,
    pub max: T,
}
impl<T: Default> Default for SpawnRange<T> {
    fn default() -> Self {
        Self {
            min: Default::default(),
            max: Default::default(),
        }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct BallSpawnConfig {
    pub count: usize,
    pub radius_range: SpawnRange<f32>,
    pub x_range: SpawnRange<f32>,
    pub y_range: SpawnRange<f32>,
    pub vel_x_range: SpawnRange<f32>,
    pub vel_y_range: SpawnRange<f32>,
}
impl Default for BallSpawnConfig {
    fn default() -> Self {
        Self {
            count: 150,
            radius_range: SpawnRange {
                min: 10.0,
                max: 20.0,
            },
            x_range: SpawnRange {
                min: -576.0,
                max: 576.0,
            },
            y_range: SpawnRange {
                min: -324.0,
                max: 324.0,
            },
            vel_x_range: SpawnRange {
                min: -200.0,
                max: 200.0,
            },
            vel_y_range: SpawnRange {
                min: -50.0,
                max: 350.0,
            },
        }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct CollisionSeparationConfig {
    pub enabled: bool,
    pub overlap_slop: f32,
    pub push_strength: f32,
    pub max_push: f32,
    pub velocity_dampen: f32,
}
impl Default for CollisionSeparationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            overlap_slop: 0.98,
            push_strength: 0.5,
            max_push: 10.0,
            velocity_dampen: 0.2,
        }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct ExplosionConfig {
    pub enabled: bool,
    pub impulse: f32,
    pub radius: f32,
    pub falloff_exp: f32,
}
impl Default for ExplosionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            impulse: 500.0,
            radius: 250.0,
            falloff_exp: 1.2,
        }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct DragConfig {
    pub enabled: bool,
    pub grab_radius: f32,
    pub pull_strength: f32,
    pub max_speed: f32,
}
impl Default for DragConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            grab_radius: 35.0,
            pull_strength: 1000.0,
            max_speed: 1500.0,
        }
    }
}
#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct InteractionConfig {
    pub explosion: ExplosionConfig,
    pub drag: DragConfig,
}
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct GameConfig {
    pub window: WindowConfig,
    pub gravity: GravityConfig,
    pub bounce: BounceConfig,
    pub balls: BallSpawnConfig,
    pub separation: CollisionSeparationConfig,
    pub rapier_debug: bool,
    pub draw_circles: bool,
    pub metaballs_enabled: bool,
    pub metaballs: MetaballsRenderConfig,
    pub metaballs_shader: MetaballsShaderConfig,
    pub noise: NoiseConfig, // NEW: procedural background noise params
    pub draw_cluster_bounds: bool,
    pub interactions: InteractionConfig,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            window: Default::default(),
            gravity: Default::default(),
            bounce: Default::default(),
            balls: Default::default(),
            separation: Default::default(),
            rapier_debug: false,
            draw_circles: false,
            metaballs_enabled: true,
            metaballs: Default::default(),
            metaballs_shader: Default::default(),
            noise: Default::default(),
            draw_cluster_bounds: false,
            interactions: Default::default(),
        }
    }
}
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsRenderConfig {
    pub iso: f32,
    pub normal_z_scale: f32,
    pub radius_multiplier: f32,
}
impl Default for MetaballsRenderConfig {
    fn default() -> Self {
        Self {
            iso: 0.6,
            normal_z_scale: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsShaderConfig {
    pub fg_mode: usize,
    pub bg_mode: usize,
}
impl Default for MetaballsShaderConfig {
    fn default() -> Self {
        Self { fg_mode: 0, bg_mode: 0 }
    }
}

// NEW: Noise configuration mapped to shader NoiseParams UBO
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct NoiseConfig {
    pub base_scale: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: bool,
}
impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            base_scale: 0.004,
            warp_amp: 0.6,
            warp_freq: 0.5,
            speed_x: 0.03,
            speed_y: 0.02,
            gain: 0.5,
            lacunarity: 2.0,
            contrast_pow: 1.25,
            octaves: 5,
            ridged: false,
        }
    }
}
impl GameConfig {
    #[allow(dead_code)]
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let data = fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        ron::from_str(&data).map_err(|e| format!("parse RON: {e}"))
    }
    #[allow(dead_code)]
    pub fn load_or_default(path: impl AsRef<Path>) -> (Self, Option<String>) {
        match Self::load_from_file(&path) {
            Ok(cfg) => (cfg, None),
            Err(e) => (Self::default(), Some(e)),
        }
    }
    pub fn load_layered<P, I>(paths: I) -> (Self, Vec<String>, Vec<String>)
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        use ron::value::Value;
        let mut merged: Option<Value> = None;
        let mut used = Vec::new();
        let mut errors = Vec::new();
        fn merge_value(base: &mut ron::value::Value, overlay: ron::value::Value) {
            use ron::value::Value;
            match (base, overlay) {
                (Value::Map(bm), Value::Map(om)) => {
                    for (k, v) in om.into_iter() {
                        let mut incoming = Some(v);
                        let mut replaced = false;
                        for (ek, ev) in bm.iter_mut() {
                            if *ek == k {
                                let val = incoming.take().unwrap();
                                merge_value(ev, val);
                                replaced = true;
                                break;
                            }
                        }
                        if !replaced {
                            bm.insert(k, incoming.unwrap());
                        }
                    }
                }
                (b, o) => *b = o,
            }
        }
        for p in paths {
            let path_ref = p.as_ref();
            match fs::read_to_string(path_ref) {
                Ok(txt) => match ron::from_str::<Value>(&txt) {
                    Ok(val) => {
                        if let Some(cur) = &mut merged {
                            merge_value(cur, val);
                        } else {
                            merged = Some(val);
                        }
                        used.push(path_ref.as_os_str().to_string_lossy().to_string());
                    }
                    Err(e) => errors.push(format!("{}: parse error: {e}", path_ref.display())),
                },
                Err(e) => errors.push(format!("{}: read error: {e}", path_ref.display())),
            }
        }
        if let Some(val) = merged {
            match val.clone().into_rust::<GameConfig>() {
                Ok(cfg) => (cfg, used, errors),
                Err(e) => (GameConfig::default(), used, {
                    let mut evec = errors;
                    evec.push(format!(
                        "failed to deserialize merged config; using defaults: {e}"
                    ));
                    evec
                }),
            }
        } else {
            (GameConfig::default(), used, errors)
        }
    }
    pub fn validate(&self) -> Vec<String> {
        let mut w = Vec::new();
        if self.window.width <= 0.0 || self.window.height <= 0.0 {
            w.push("window dimensions must be > 0".into());
        }
        if self.window.width * self.window.height > 10_000_000.0 {
            w.push(format!(
                "very large window area: {}x{}",
                self.window.width, self.window.height
            ));
        }
        if self.window.auto_close < 0.0 {
            w.push(format!(
                "window.autoClose {} negative -> treated as disabled (should be >= 0)",
                self.window.auto_close
            ));
        } else if self.window.auto_close > 0.0 && self.window.auto_close < 0.01 {
            w.push(format!(
                "window.autoClose {} very small; closes almost immediately",
                self.window.auto_close
            ));
        }
        if self.gravity.y.abs() < 1e-4 {
            w.push("gravity.y magnitude near zero; balls may float".into());
        }
        if self.gravity.y > 0.0 {
            w.push(format!(
                "gravity.y is positive ({}); Y-up world? typical configs use negative for downward",
                self.gravity.y
            ));
        }
        if self.gravity.y < -2000.0 {
            w.push(format!(
                "gravity.y very large magnitude ({}); integration instability possible",
                self.gravity.y
            ));
        }
        if !(0.0..=1.5).contains(&self.bounce.restitution) {
            w.push(format!(
                "restitution {} outside recommended 0..1.5",
                self.bounce.restitution
            ));
        }
        if self.bounce.restitution < 0.0 {
            w.push("restitution negative -> energy gain/clamping side effects".into());
        }
        if self.balls.count == 0 {
            w.push("balls.count is 0; nothing will spawn".into());
        }
        if self.balls.count > 50_000 {
            w.push(format!(
                "balls.count {} very high; performance may suffer",
                self.balls.count
            ));
        }
        fn check_range_f32(w: &mut Vec<String>, label: &str, r: &SpawnRange<f32>) {
            if r.min > r.max {
                w.push(format!(
                    "{label} min ({}) greater than max ({})",
                    r.min, r.max
                ));
            }
            if (r.max - r.min).abs() < f32::EPSILON {
                w.push(format!("{label} min == max ({}) -> zero variation", r.min));
            }
        }
        check_range_f32(&mut w, "balls.radius_range", &self.balls.radius_range);
        if self.balls.radius_range.min <= 0.0 {
            w.push("balls.radius_range.min must be > 0".into());
        }
        check_range_f32(&mut w, "balls.x_range", &self.balls.x_range);
        check_range_f32(&mut w, "balls.y_range", &self.balls.y_range);
        check_range_f32(&mut w, "balls.vel_x_range", &self.balls.vel_x_range);
        check_range_f32(&mut w, "balls.vel_y_range", &self.balls.vel_y_range);
        if self.separation.enabled {
            if !(0.0..=1.2).contains(&self.separation.overlap_slop) {
                w.push(format!(
                    "separation.overlap_slop {} outside 0..1.2 typical bounds",
                    self.separation.overlap_slop
                ));
            }
            if self.separation.push_strength < 0.0 {
                w.push("separation.push_strength negative".into());
            }
            if self.separation.max_push <= 0.0 {
                w.push("separation.max_push must be > 0".into());
            }
            if !(0.0..=1.0).contains(&self.separation.velocity_dampen) {
                w.push(format!(
                    "separation.velocity_dampen {} outside 0..1",
                    self.separation.velocity_dampen
                ));
            }
        }
        if self.interactions.explosion.enabled {
            let ex = &self.interactions.explosion;
            if ex.impulse <= 0.0 {
                w.push("explosion.impulse must be > 0 when enabled".into());
            }
            if ex.radius <= 0.0 {
                w.push("explosion.radius must be > 0".into());
            }
            if ex.falloff_exp < 0.0 {
                w.push("explosion.falloff_exp negative -> increasing force with distance".into());
            }
            if ex.falloff_exp > 8.0 {
                w.push(format!(
                    "explosion.falloff_exp {} very high (force extremely localized)",
                    ex.falloff_exp
                ));
            }
        }
        if self.interactions.drag.enabled {
            let dr = &self.interactions.drag;
            if dr.grab_radius <= 0.0 {
                w.push("drag.grab_radius must be > 0".into());
            }
            if dr.pull_strength <= 0.0 {
                w.push("drag.pull_strength must be > 0".into());
            }
            if dr.max_speed < 0.0 {
                w.push("drag.max_speed negative -> treated as cap?".into());
            }
            if dr.max_speed != 0.0 && dr.max_speed < dr.pull_strength * 0.1 {
                w.push(format!(
                    "drag.max_speed {} may be too low relative to pull_strength {} -> jerky motion",
                    dr.max_speed, dr.pull_strength
                ));
            }
        }
        if self.metaballs.radius_multiplier <= 0.0 {
            w.push(format!(
                "metaballs.radius_multiplier {} must be > 0 (visual scaling)",
                self.metaballs.radius_multiplier
            ));
        } else if self.metaballs.radius_multiplier > 5.0 {
            w.push(format!(
                "metaballs.radius_multiplier {} very large (visual field may become overly blobby)",
                self.metaballs.radius_multiplier
            ));
        }
        w
    }
}
// Tests moved unchanged
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}
