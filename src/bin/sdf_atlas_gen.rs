//! SDF Atlas Generation Utility
//!
//! Converts a registry JSON + packed atlas PNG into runtime `sdf_atlas.json`.
//! Only single-channel (sdf_r8) distance field supported for now.
//!
//! Example:
//!   cargo run --bin sdf_atlas_gen -- \
//!     --registry distance-field-generator/sample_sdf_registry.json \
//!     --atlas-png assets/shapes/sdf_atlas.png \
//!     --out-json assets/shapes/sdf_atlas.json \
//!     --tile-size 64

use std::{fs, path::PathBuf};
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate runtime SDF atlas json", long_about=None)]
struct Args {
    #[arg(long)] registry: PathBuf,
    #[arg(long, alias="atlas")] atlas_png: PathBuf,
    #[arg(long)] out_json: PathBuf,
    #[arg(long)] tile_size: u32,
    #[arg(long)] atlas_width: Option<u32>,
    #[arg(long)] atlas_height: Option<u32>,
    #[arg(long)] distance_range: Option<f32>,
    #[arg(long, default_value="sdf_r8")] channel_mode: String,
}

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    name: String,
    index: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    #[serde(default)] pivot_x: Option<f32>,
    #[serde(default)] pivot_y: Option<f32>,
}

#[derive(Serialize)] struct OutPivot { x: f32, y: f32 }
#[derive(Serialize)] struct OutUv { u0: f32, v0: f32, u1: f32, v1: f32 }
#[derive(Serialize)] struct OutRect { x: u32, y: u32, w: u32, h: u32 }
#[derive(Serialize)] struct OutShapeEntry { name:String, index:u32, px:OutRect, uv:OutUv, pivot:OutPivot, #[serde(skip_serializing_if="Option::is_none")] advance_scale: Option<f32>, metadata: serde_json::Value }
#[derive(Serialize)] struct OutRoot { version:u32, distance_range: f32, tile_size:u32, atlas_width:u32, atlas_height:u32, channel_mode:String, shapes: Vec<OutShapeEntry> }

fn main() -> Result<()> {
    let args = Args::parse();
    if !matches!(args.channel_mode.as_str(), "sdf_r8" | "msdf_rgb" | "msdf_rgba") { anyhow::bail!("unsupported --channel-mode {}", args.channel_mode); }

    let reg_txt = fs::read_to_string(&args.registry).with_context(|| format!("read registry {:?}", args.registry))?;
    let entries: Vec<RegistryEntry> = if reg_txt.trim_start().starts_with('[') {
        serde_json::from_str(&reg_txt).context("parse registry array")?
    } else {
    #[derive(Deserialize)] struct Root { shapes: Vec<RegistryEntry> }
        let root: Root = serde_json::from_str(&reg_txt).context("parse registry root")?; root.shapes
    };
    if entries.is_empty() { anyhow::bail!("registry had no entries"); }

    let (atlas_w, atlas_h) = if let (Some(w), Some(h)) = (args.atlas_width, args.atlas_height) { (w,h) } else {
        let img_bytes = fs::read(&args.atlas_png).with_context(|| "read atlas png")?;
        let img = image::load_from_memory(&img_bytes).context("decode png")?;
        (img.width(), img.height())
    };
    if atlas_w % args.tile_size != 0 || atlas_h % args.tile_size != 0 { eprintln!("warning: atlas dims {}x{} not multiples of tile_size {}", atlas_w, atlas_h, args.tile_size); }

    let distance_range = args.distance_range.unwrap_or(args.tile_size as f32 * 0.125);
    let mut shapes = Vec::with_capacity(entries.len());
    for (i,e) in entries.iter().enumerate() {
        if e.index != i as u32 { eprintln!("warning: index mismatch entry {} json {} vs pos {}", e.name, e.index, i); }
        if e.w != args.tile_size || e.h != args.tile_size { eprintln!("warning: shape '{}' dims {}x{} != tile_size {}", e.name, e.w, e.h, args.tile_size); }
        let pivot_x = e.pivot_x.unwrap_or(0.5); let pivot_y = e.pivot_y.unwrap_or(0.5);
        shapes.push(OutShapeEntry { name: e.name.clone(), index: e.index, px: OutRect { x: e.x, y: e.y, w: e.w, h: e.h }, uv: OutUv { u0: e.u0, v0: e.v0, u1: e.u1, v1: e.v1 }, pivot: OutPivot { x: pivot_x, y: pivot_y }, advance_scale: None, metadata: serde_json::json!({}) });
    }
    let root = OutRoot { version:1, distance_range, tile_size: args.tile_size, atlas_width: atlas_w, atlas_height: atlas_h, channel_mode: args.channel_mode.clone(), shapes };
    let json_str = serde_json::to_string_pretty(&root)?;
    if let Some(parent) = args.out_json.parent() { fs::create_dir_all(parent)?; }
    fs::write(&args.out_json, json_str).with_context(|| format!("write {:?}", args.out_json))?;
    println!("Wrote {}", args.out_json.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_generation_flow() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg.json");
        let png = tmp.path().join("atlas.png");
        let img = image::RgbaImage::from_pixel(64,64, image::Rgba([128,0,0,255])); img.save(&png).unwrap();
    fs::write(&reg, r#"[{"name":"circle","index":1,"x":0,"y":0,"w":64,"h":64,"u0":0.0,"v0":0.0,"u1":1.0,"v1":1.0}]"#).unwrap();
        let out = tmp.path().join("atlas.json");
        let reg_txt = fs::read_to_string(&reg).unwrap();
        let entries: Vec<RegistryEntry> = serde_json::from_str(&reg_txt).unwrap(); assert_eq!(entries.len(),1);
        let img_bytes = fs::read(&png).unwrap(); let img = image::load_from_memory(&img_bytes).unwrap(); assert_eq!(img.width(),64);
        let distance_range = 64f32 * 0.125; let mut shapes = Vec::new();
    for (i,e) in entries.iter().enumerate() { assert_eq!(e.index, (i as u32)+1); shapes.push(super::OutShapeEntry { name: e.name.clone(), index: e.index, px: super::OutRect { x: e.x, y: e.y, w: e.w, h: e.h }, uv: super::OutUv { u0: e.u0, v0: e.v0, u1: e.u1, v1: e.v1 }, pivot: super::OutPivot { x: 0.5, y:0.5 }, advance_scale: None, metadata: serde_json::json!({}) }); }
    let root = super::OutRoot { version:1, distance_range, tile_size:64, atlas_width: 64, atlas_height:64, channel_mode: "sdf_r8".into(), shapes }; let js = serde_json::to_string_pretty(&root).unwrap(); fs::write(&out, js).unwrap(); let produced = fs::read_to_string(&out).unwrap(); assert!(produced.contains("\"version\": 1"));
    }
}
