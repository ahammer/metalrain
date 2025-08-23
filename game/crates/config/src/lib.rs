// Phase 1: Ported legacy configuration (pure data crate; no Bevy dependency).
// Provides: data structures, layered loading, validation producing warnings (non-fatal), and tests.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct GravityConfig {
    pub y: f32,
}
impl Default for GravityConfig {
    fn default() -> Self {
        Self { y: -600.0 }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct BounceConfig {
    pub restitution: f32,
}
impl Default for BounceConfig {
    fn default() -> Self {
        Self { restitution: 0.85 }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct InteractionConfig {
    pub explosion: ExplosionConfig,
    pub drag: DragConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsRenderConfig {
    pub iso: f32,
    pub normal_z_scale: f32,
    /// Multiplier applied only for metaballs distance field (visual).
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct EmitterConfig {
    pub enabled: bool,
    /// Approximate spawn rate per second (interpreted relative to a nominal 60 fps in deterministic tests).
    pub rate_per_sec: f32,
    /// Upper bound on total concurrently live balls spawned by the emitter (including initial ring).
    pub max_live: usize,
    /// Optional burst size spawned immediately on startup (0 = none).
    pub burst: usize,
}
impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rate_per_sec: 30.0,
            max_live: 5_000,
            burst: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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
    pub emitter: EmitterConfig,
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
            emitter: Default::default(),
            draw_cluster_bounds: false,
            interactions: Default::default(),
        }
    }
}

impl GameConfig {
    /// Load from a single RON file (errors contain human-readable context).
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let data = fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        ron::from_str(&data).map_err(|e| format!("parse RON: {e}"))
    }

    /// Load file; on failure returns default config plus error string.
    pub fn load_or_default(path: impl AsRef<Path>) -> (Self, Option<String>) {
        match Self::load_from_file(&path) {
            Ok(cfg) => (cfg, None),
            Err(e) => (Self::default(), Some(e)),
        }
    }

    /// Load multiple layers; later overrides earlier (deep merge).
    /// Skips missing files; returns (config, used_paths, errors).
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
                Err(e) => {
                    let mut evec = errors;
                    evec.push(format!(
                        "failed to deserialize merged config; using defaults: {e}"
                    ));
                    (GameConfig::default(), used, evec)
                }
            }
        } else {
            (GameConfig::default(), used, errors)
        }
    }

    /// Produce validation warnings (non-fatal) for suspicious values.
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
                "gravity.y is positive ({}); typical configs use negative for downward",
                self.gravity.y
            ));
        }
        if self.gravity.y < -2000.0 {
            w.push(format!(
                "gravity.y very large magnitude ({}); instability possible",
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
                w.push(format!("{label} min ({}) greater than max ({})", r.min, r.max));
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
                    "separation.overlap_slop {} outside 0..1.2",
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
                w.push("explosion.falloff_exp negative".into());
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
                "metaballs.radius_multiplier {} must be > 0",
                self.metaballs.radius_multiplier
            ));
        } else if self.metaballs.radius_multiplier > 5.0 {
            w.push(format!(
                "metaballs.radius_multiplier {} very large (visual field may be overly blobby)",
                self.metaballs.radius_multiplier
            ));
        }
        if self.emitter.enabled {
            if self.emitter.rate_per_sec <= 0.0 {
                w.push("emitter.rate_per_sec must be > 0 when enabled".into());
            }
            if self.emitter.max_live == 0 {
                w.push("emitter.max_live is 0; no entities can spawn".into());
            }
            if self.emitter.burst > self.emitter.max_live {
                w.push("emitter.burst exceeds emitter.max_live; burst capped at max_live".into());
            }
            if self.emitter.rate_per_sec > 10_000.0 {
                w.push(format!(
                    "emitter.rate_per_sec {} extremely high; performance risk",
                    self.emitter.rate_per_sec
                ));
            }
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
        )"#;
        let cfg = GameConfig::load_from_file(write_temp(sample).path()).expect("parse config");
        assert_eq!(cfg.window.width, 800.0);
        assert_eq!(cfg.balls.count, 10);
        assert_eq!(cfg.bounce.restitution, 0.5);
        assert!((cfg.metaballs.iso - 0.55).abs() < 1e-6);
        assert!(cfg.validate().is_empty(), "expected no warnings");
    }

    #[test]
    fn validate_detects_warnings() {
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
                radius_range: SpawnRange { min: 0.0, max: 0.0 },
                x_range: SpawnRange { min: 10.0, max: -10.0 },
                y_range: SpawnRange { min: 1.0, max: 1.0 },
                vel_x_range: SpawnRange { min: 5.0, max: 1.0 },
                vel_y_range: SpawnRange { min: 0.0, max: 0.0 },
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
            metaballs: MetaballsRenderConfig {
                iso: 0.6,
                normal_z_scale: 1.0,
                radius_multiplier: 1.0,
            },
            emitter: EmitterConfig::default(),
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
        };
        let warnings = bad.validate();
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
        let (cfg, used, errors) = GameConfig::load_layered([
            write_temp(base).path().to_path_buf(),
            write_temp(override_one).path().to_path_buf(),
        ]);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(used.len(), 2);
        assert_eq!(cfg.window.width, 900.0);
        assert_eq!(cfg.window.title, "Custom Title");
        assert_eq!(cfg.bounce.restitution, 1.1);
        assert_eq!(cfg.window.height, WindowConfig::default().height);
    }

    #[test]
    fn load_or_default_existing_file() {
        let sample = r"(window: (width: 640.0, height: 360.0), gravity: (y: -500.0))";
        let (cfg, err) = GameConfig::load_or_default(write_temp(sample).path());
        assert!(err.is_none());
        assert_eq!(cfg.window.width, 640.0);
        assert_eq!(cfg.gravity.y, -500.0);
    }

    #[test]
    fn parse_autoclose_and_validate() {
        let sample = r"(window: (autoClose: 3.25), gravity: (y: -600.0))";
        let cfg = GameConfig::load_from_file(write_temp(sample).path()).expect("parse config");
        assert!((cfg.window.auto_close - 3.25).abs() < 1e-6);

        let neg_sample = r"(window: (autoClose: -5.0))";
        let cfg2 = GameConfig::load_from_file(write_temp(neg_sample).path()).expect("parse config");
        assert!(
            cfg2.validate().iter().any(|w| w.contains("window.autoClose")),
            "expected warning for negative autoClose"
        );
    }

    // Helper: create a temp file with given contents; returns handle (kept for lifetime)
    fn write_temp(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("tmp");
        f.write_all(contents.as_bytes()).unwrap();
        f
    }
}
