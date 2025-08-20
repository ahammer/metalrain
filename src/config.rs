use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub title: String,
    /// Automatically close the app after this many seconds. 0.0 (or omitted) = run indefinitely.
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
    pub draw_cluster_bounds: bool,
    pub interactions: InteractionConfig,
    pub fluid_sim: FluidSimConfig,
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
            draw_cluster_bounds: false,
            interactions: Default::default(),
            fluid_sim: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsRenderConfig {
    pub iso: f32,
    pub normal_z_scale: f32,
    /// Multiplier applied to each physical ball radius ONLY for the metaballs distance field (visual expansion / contraction).
    /// Does not change physics or circle debug rendering. Values >1.0 make blobs fuse earlier; <1.0 tighten them.
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
pub struct FluidSimConfig {
    /// Horizontal grid resolution (number of cells / pixels). Powers of two typical but not required.
    pub width: u32,
    /// Vertical grid resolution.
    pub height: u32,
    /// Jacobi pressure iterations per frame (higher -> less divergence, more cost).
    pub jacobi_iterations: u32,
    /// Simulation time step seconds (clamped internally to <= 0.033 for stability).
    pub time_step: f32,
    /// Scalar dye dissipation per step (0..1, closer to 1 retains color longer).
    pub dissipation: f32,
    /// Velocity field dissipation per step (0..1, closer to 1 retains motion longer).
    pub velocity_dissipation: f32,
    /// Strength of injected force when user drags / interacts.
    pub force_strength: f32,
    /// Master enable; when false the FluidSimPlugin may skip heavy work / allocation (future optimization).
    pub enabled: bool,
    // --- Impulse (ball wake) parameters (Phase 4) ---
    /// Minimum speed factor relative to force_strength to emit an impulse (speed < force_strength * factor => skip)
    pub impulse_min_speed_factor: f32,
    /// World-space radius scale applied to ball speed to derive base impulse radius before clamping.
    pub impulse_radius_scale: f32,
    /// Minimum / maximum world-space radius clamp applied before mapping to grid space.
    pub impulse_radius_world_min: f32,
    pub impulse_radius_world_max: f32,
    /// Multiplier converting ball speed to impulse base strength.
    pub impulse_strength_scale: f32,
    /// Falloff exponent n in (1 - r/R)^n for both velocity + dye.
    pub impulse_falloff_exponent: f32,
    /// Dye injection global multiplier applied after strength & falloff.
    pub impulse_dye_scale: f32,
    /// Optional debug amplification factor applied to impulse strength (not persisted if 1.0)
    pub impulse_debug_strength_mul: f32,
    /// Optional debug flag to spawn a periodic central test impulse
    pub impulse_debug_test_enabled: bool,
    /// When true, seed the dye texture with random blotches at startup for motion visibility. Disable to view only ball‑injected dye.
    pub seed_initial_dye: bool,
}
impl Default for FluidSimConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            jacobi_iterations: 20,
            time_step: 1.0/60.0,
            dissipation: 0.995,
            velocity_dissipation: 0.999,
            force_strength: 120.0,
            enabled: true,
            impulse_min_speed_factor: 0.05,
            impulse_radius_scale: 2.0,
            impulse_radius_world_min: 8.0,
            impulse_radius_world_max: 96.0,
            impulse_strength_scale: 0.4,
            impulse_falloff_exponent: 2.0,
            impulse_dye_scale: 0.15,
            impulse_debug_strength_mul: 1.0,
            impulse_debug_test_enabled: false,
            seed_initial_dye: true,
        }
    }
}

impl GameConfig {
    // These legacy single-file helpers are retained for potential direct usage in tools & tests.
    // The layered loader is preferred in production startup path.
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

    /// Load multiple config layers, later files overriding earlier ones (shallow & deep merge).
    /// Missing files are skipped; returns (config, list_of_layer_paths_used, list_of_errors).
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

    /// Validate the configuration returning a list of human‑readable warning strings.
    /// These represent suspicious / potentially unintended values but are not hard errors.
    /// Call at startup and log each warning with `warn!`.
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
            w.push(format!("window.autoClose {} negative -> treated as disabled (should be >= 0)", self.window.auto_close));
        } else if self.window.auto_close > 0.0 && self.window.auto_close < 0.01 {
            w.push(format!("window.autoClose {} very small; closes almost immediately", self.window.auto_close));
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
        // Fluid sim validation
        if self.fluid_sim.enabled {
            if self.fluid_sim.width < 8 || self.fluid_sim.height < 8 {
                w.push("fluid_sim width/height must be >= 8".into());
            }
            if self.fluid_sim.width > 4096 || self.fluid_sim.height > 4096 {
                w.push(format!("fluid_sim resolution {}x{} very large; VRAM heavy", self.fluid_sim.width, self.fluid_sim.height));
            }
            if self.fluid_sim.jacobi_iterations == 0 {
                w.push("fluid_sim.jacobi_iterations is 0 -> no pressure solve (divergence)".into());
            } else if self.fluid_sim.jacobi_iterations > 200 { w.push(format!("fluid_sim.jacobi_iterations {} extremely high", self.fluid_sim.jacobi_iterations)); }
            if !(0.0..=0.1).contains(&self.fluid_sim.time_step) { w.push(format!("fluid_sim.time_step {} outside 0..0.1 typical", self.fluid_sim.time_step)); }
            if !(0.0..=1.0).contains(&self.fluid_sim.dissipation) { w.push(format!("fluid_sim.dissipation {} outside 0..1", self.fluid_sim.dissipation)); }
            if !(0.0..=1.0).contains(&self.fluid_sim.velocity_dissipation) { w.push(format!("fluid_sim.velocity_dissipation {} outside 0..1", self.fluid_sim.velocity_dissipation)); }
            if self.fluid_sim.force_strength < 0.0 { w.push("fluid_sim.force_strength negative".into()); }
        }
        w
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_sample_config() {
        let sample = r#"(
            window: (width: 800.0, height: 600.0, title: "Test"),
            gravity: (y: -9.8),
            bounce: (restitution: 0.5),
            balls: (
                count: 10,
                radius_range: (min: 1.0, max: 2.0),
                x_range: (min: -10.0, max: 10.0),
                y_range: (min: -5.0, max: 5.0),
                vel_x_range: (min: -1.0, max: 1.0),
                vel_y_range: (min: 0.0, max: 2.0),
            ),
            separation: (
                enabled: true,
                overlap_slop: 0.98,
                push_strength: 0.5,
                max_push: 10.0,
                velocity_dampen: 0.2,
            ),
            rapier_debug: false,
            draw_circles: true,
            metaballs_enabled: true,
            metaballs: (
                iso: 0.55,
                normal_z_scale: 1.1,
                radius_multiplier: 1.25,
            ),
            draw_cluster_bounds: false,
            interactions: (
                explosion: (enabled: true, impulse: 500.0, radius: 200.0, falloff_exp: 1.0),
                drag: (enabled: true, grab_radius: 30.0, pull_strength: 800.0, max_speed: 1200.0),
            ),
            fluid_sim: (
                width: 192,
                height: 128,
                jacobi_iterations: 30,
                time_step: 0.014,
                dissipation: 0.99,
                velocity_dissipation: 0.995,
                force_strength: 200.0,
                enabled: true,
            ),
        )"#;
        let mut file = tempfile::NamedTempFile::new().expect("tmp file");
        file.write_all(sample.as_bytes()).unwrap();
        let cfg = GameConfig::load_from_file(file.path()).expect("parse config");
        assert_eq!(cfg.window.width, 800.0);
        assert_eq!(cfg.balls.count, 10);
        assert_eq!(cfg.bounce.restitution, 0.5);
    assert!((cfg.metaballs.iso - 0.55).abs() < 1e-6);
    assert_eq!(cfg.fluid_sim.width, 192);
    assert_eq!(cfg.fluid_sim.jacobi_iterations, 30);
    assert!((cfg.fluid_sim.time_step - 0.014).abs() < 1e-6);
    // hard_cluster_boundaries removed in simplified flat-color renderer
        // Should produce no warnings for the nominal sample config
        assert!(
            cfg.validate().is_empty(),
            "expected no validation warnings for sample config"
        );
    }

    #[test]
    fn validate_detects_warnings() {
        // Intentionally craft a config with multiple issues
        let bad = GameConfig {
            window: WindowConfig {
                width: -100.0,
                height: 0.0,
                title: "Bad".into(),
                auto_close: 0.0,
            },
            gravity: GravityConfig { y: 0.0 },
            bounce: BounceConfig { restitution: -0.2 },
            balls: BallSpawnConfig {
                count: 0,
                radius_range: SpawnRange { min: 0.0, max: 0.0 }, // zero + invalid min
                x_range: SpawnRange {
                    min: 10.0,
                    max: -10.0,
                }, // inverted
                y_range: SpawnRange { min: 1.0, max: 1.0 },      // zero variation
                vel_x_range: SpawnRange { min: 5.0, max: 1.0 },  // inverted
                vel_y_range: SpawnRange { min: 0.0, max: 0.0 },  // zero
            },
            separation: CollisionSeparationConfig {
                enabled: true,
                overlap_slop: 2.0,
                push_strength: -1.0,
                max_push: 0.0,
                velocity_dampen: 1.5,
            },
            rapier_debug: false,
            draw_circles: true,
            metaballs_enabled: true,
            metaballs: MetaballsRenderConfig { iso: 0.6, normal_z_scale: 1.0, radius_multiplier: 1.0 },
            draw_cluster_bounds: false,
            interactions: InteractionConfig {
                explosion: ExplosionConfig {
                    enabled: true,
                    impulse: 0.0,
                    radius: -10.0,
                    falloff_exp: -1.0,
                },
                drag: DragConfig {
                    enabled: true,
                    grab_radius: 0.0,
                    pull_strength: 0.0,
                    max_speed: -5.0,
                },
            },
            fluid_sim: FluidSimConfig { width: 0, height: 10, jacobi_iterations: 0, time_step: 0.2, dissipation: 2.0, velocity_dissipation: -0.5, force_strength: -10.0, enabled: true,
                impulse_min_speed_factor: 0.05, impulse_radius_scale: 2.0, impulse_radius_world_min: 1.0, impulse_radius_world_max: 2.0,
                impulse_strength_scale: 0.4, impulse_falloff_exponent: 2.0, impulse_dye_scale: 0.15, impulse_debug_strength_mul: 1.0, impulse_debug_test_enabled: false, seed_initial_dye: true },
        };
        let warnings = bad.validate();
        // Ensure a representative subset of expected warnings are present
        let joined = warnings.join(" | ");
        assert!(joined.contains("window dimensions must be > 0"));
        assert!(joined.contains("gravity.y magnitude near zero"));
        assert!(joined.contains("restitution negative"));
        assert!(joined.contains("balls.count is 0"));
        assert!(joined.contains("balls.radius_range.min must be > 0"));
        assert!(joined.contains("balls.radius_range min == max"));
        assert!(joined.contains("balls.x_range min (10"));
        assert!(joined.contains("separation.overlap_slop"));
        assert!(joined.contains("separation.velocity_dampen"));
        assert!(joined.contains("explosion.impulse must be > 0"));
        assert!(joined.contains("drag.max_speed negative"));
        assert!(
            warnings.len() >= 12,
            "expected many warnings, got {}: {joined}",
            warnings.len()
        );
    }

    #[test]
    fn load_or_default_missing_file() {
        let (cfg, err) = GameConfig::load_or_default("this/file/does/not/exist.ron");
        assert!(err.is_some());
        // Defaults applied
        assert_eq!(cfg.window.width, WindowConfig::default().width);
    }

    #[test]
    fn layered_merge_overrides() {
        let base = r"(
            window: (width: 900.0),
            gravity: (y: -700.0),
            bounce: (restitution: 0.7),
        )";
        let override_one = r#"(
            window: (title: "Custom Title"),
            bounce: (restitution: 1.1),
        )"#;
        let mut f1 = tempfile::NamedTempFile::new().unwrap();
        let mut f2 = tempfile::NamedTempFile::new().unwrap();
        f1.write_all(base.as_bytes()).unwrap();
        f2.write_all(override_one.as_bytes()).unwrap();
        let (cfg, used, errors) = GameConfig::load_layered([f1.path(), f2.path()]);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(used.len(), 2);
        assert_eq!(cfg.window.width, 900.0); // from base
        assert_eq!(cfg.window.title, "Custom Title"); // overridden
        assert_eq!(cfg.bounce.restitution, 1.1); // overridden
                                                 // Height default still present
        assert_eq!(cfg.window.height, WindowConfig::default().height);
    }

    #[test]
    fn load_or_default_existing_file() {
        let sample = r"(window: (width: 640.0, height: 360.0), gravity: (y: -500.0))";
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(sample.as_bytes()).unwrap();
        let (cfg, err) = GameConfig::load_or_default(file.path());
        assert!(err.is_none());
        assert_eq!(cfg.window.width, 640.0);
        assert_eq!(cfg.gravity.y, -500.0);
    }

    #[test]
    fn parse_autoclose_and_validate() {
        // explicit positive value
        let sample = r"(window: (autoClose: 3.25), gravity: (y: -600.0))";
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(sample.as_bytes()).unwrap();
        let cfg = GameConfig::load_from_file(file.path()).expect("parse config");
        assert!((cfg.window.auto_close - 3.25).abs() < 1e-6);
        // negative -> warning
        let neg_sample = r"(window: (autoClose: -5.0))";
        let mut file2 = tempfile::NamedTempFile::new().unwrap();
        file2.write_all(neg_sample.as_bytes()).unwrap();
        let cfg2 = GameConfig::load_from_file(file2.path()).expect("parse config");
        assert!(cfg2.validate().iter().any(|w| w.contains("window.autoClose")), "expected warning for negative autoClose");
    }
}
