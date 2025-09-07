//! Procedural SDF Atlas Builder
//!
//! Generates a simple SDF atlas PNG + JSON for a small library of shapes:
//!   - circle, triangle (equilateral), square
//!   - glyphs 0-9, A-Z rendered via a crude vector approximation (square + cutouts for now)
//! This is a bootstrap tool so artists / future pipeline can be replaced later.
//!
//! Usage:
//!   cargo run --bin sdf_atlas_build -- \
//!       --out-png assets/shapes/sdf_atlas.png \
//!       --out-json assets/shapes/sdf_atlas.json \
//!       --tile-size 64
//!
//! The JSON produced is directly ingestible by `SdfAtlas` loader (version=1, indices 1..N).
//! Indices are assigned in insertion order starting at 1 (0 reserved sentinel).

use std::{fs, path::PathBuf};
use clap::Parser;
use serde::Serialize;
use image::{ImageBuffer, Luma};
use bevy::prelude::Vec2; // reuse math type already in dependency graph

#[derive(Parser, Debug)]
#[command(about="Procedurally generate an SDF atlas", version, author)]
struct Args {
    #[arg(long)] out_png: PathBuf,
    #[arg(long)] out_json: PathBuf,
    #[arg(long, default_value = "64")] tile_size: u32,
    #[arg(long, default_value = "sdf_r8")] channel_mode: String,
    #[arg(long, default_value_t = 0.125)] distance_span_factor: f32, // distance_range = tile_size * factor
}

#[derive(Serialize)] struct OutPivot { x: f32, y: f32 }
#[derive(Serialize)] struct OutUv { u0: f32, v0: f32, u1: f32, v1: f32 }
#[derive(Serialize)] struct OutRect { x: u32, y: u32, w: u32, h: u32 }
#[derive(Serialize)] struct OutShapeEntry { name:String, index:u32, px:OutRect, uv:OutUv, pivot:OutPivot, #[serde(skip_serializing_if="Option::is_none")] advance_scale: Option<f32>, metadata: serde_json::Value }
#[derive(Serialize)] struct OutRoot { version:u32, distance_range:f32, tile_size:u32, atlas_width:u32, atlas_height:u32, channel_mode:String, shapes:Vec<OutShapeEntry> }

fn sdf_circle(x:f32,y:f32){/* doc stub for future docs generator */}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.tile_size < 16 { anyhow::bail!("tile_size too small (min 16)"); }
    if !matches!(args.channel_mode.as_str(), "sdf_r8"|"msdf_rgb"|"msdf_rgba") { anyhow::bail!("unsupported channel_mode {}",&args.channel_mode); }

    // Shape catalog order determines index assignment (start at 1)
    let mut shape_names: Vec<String> = vec!["circle".into(), "triangle".into(), "square".into()];
    for c in '0'..='9' { shape_names.push(format!("glyph_{}", c)); }
    for c in 'A'..='Z' { shape_names.push(format!("glyph_{}", c)); }

    let tile = args.tile_size as i32;
    let grid_cols = 8i32; // adjustable
    let grid_rows = ((shape_names.len() as i32 + grid_cols - 1) / grid_cols).max(1);
    let atlas_w = (grid_cols as u32) * args.tile_size;
    let atlas_h = (grid_rows as u32) * args.tile_size;

    let mut atlas: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_pixel(atlas_w, atlas_h, Luma([128u8])); // neutral 0 distance

    // Distance normalization: encode signed distance in [0,255] with 128 = 0
    let max_d = args.tile_size as f32 * 0.5; // conservative half-tile support

    for (idx, name) in shape_names.iter().enumerate() {
        let col = (idx as i32) % grid_cols;
        let row = (idx as i32) / grid_cols;
        let ox = col * tile;
        let oy = row * tile;
        for py in 0..tile { for px in 0..tile { let fx = (px as f32 + 0.5) / args.tile_size as f32; let fy = (py as f32 + 0.5) / args.tile_size as f32; let cx = fx * 2.0 - 1.0; let cy = fy * 2.0 - 1.0; let sd = signed_distance(name, cx, cy); let d_clamped = sd.clamp(-max_d, max_d); let n = 0.5 + 0.5 * (d_clamped / max_d); let u8v = (n * 255.0).round().clamp(0.0,255.0) as u8; atlas.put_pixel((ox + px) as u32, (oy + py) as u32, Luma([u8v])); }}
    }

    if let Some(parent) = args.out_png.parent() { fs::create_dir_all(parent)?; }
    atlas.save(&args.out_png)?;

    // Build JSON metadata entries
    let mut shapes_meta = Vec::new();
    for (i,name) in shape_names.iter().enumerate() { let col = (i as i32) % grid_cols; let row = (i as i32) / grid_cols; let x = (col * tile) as u32; let y = (row * tile) as u32; let uv0 = x as f32 / atlas_w as f32; let uv1 = (x + args.tile_size) as f32 / atlas_w as f32; let v0 = y as f32 / atlas_h as f32; let v1 = (y + args.tile_size) as f32 / atlas_h as f32; shapes_meta.push(OutShapeEntry { name: name.clone(), index: (i+1) as u32, px: OutRect { x, y, w: args.tile_size, h: args.tile_size }, uv: OutUv { u0: uv0, v0, u1: uv1, v1 }, pivot: OutPivot { x:0.5, y:0.5 }, advance_scale: None, metadata: serde_json::json!({}) }); }
    let distance_range = (args.tile_size as f32 * args.distance_span_factor).max(1.0);
    let root = OutRoot { version:1, distance_range, tile_size: args.tile_size, atlas_width: atlas_w, atlas_height: atlas_h, channel_mode: args.channel_mode, shapes: shapes_meta };
    let json_str = serde_json::to_string_pretty(&root)?; if let Some(p) = args.out_json.parent() { fs::create_dir_all(p)?; } fs::write(&args.out_json, json_str)?;
    println!("Generated procedural SDF atlas: {} ({}x{})", args.out_png.display(), atlas_w, atlas_h);
    Ok(())
}

fn signed_distance(name:&str, x: f32, y: f32) -> f32 {
    // x,y in [-1,1]
    match name {
        "circle" => { let r = 0.8; (r - (x*x + y*y).sqrt()) * (0.5 * r) }
        "square" => { let s = 0.8; let dx = (x.abs() - s).max(0.0); let dy = (y.abs() - s).max(0.0); let outside = (dx*dx + dy*dy).sqrt(); let inside = (s - x.abs()).min(s - y.abs()); if dx > 0.0 || dy > 0.0 { -outside } else { inside } }
        "triangle" => { sdf_equilateral_triangle(x,y) }
        _ if name.starts_with("glyph_") => sdf_glyph_placeholder(name, x, y),
        _ => 0.0,
    }
}

fn sdf_equilateral_triangle(x:f32,y:f32)->f32{ // side length normalized inside [-1,1]
    let k = (3.0f32).sqrt();
    let p = Vec2::new(x, y);
    let mut q = Vec2::new(p.x.abs() - 0.8, p.y + 0.8/k);
    if q.x + k*q.y > 0.0 { let tmp_x = (q.x - k*q.y)/2.0; let tmp_y = (-k*q.x - q.y)/2.0; q = Vec2::new(tmp_x, tmp_y); }
    q.x = q.x.max(0.0); q.y = q.y.max(0.0);
    -q.length() * q.y.signum() - (p.y + 0.8/k).min(0.0)
}

fn sdf_glyph_placeholder(name:&str, x:f32,y:f32)->f32{ // Simple block glyph with cutouts for variation
    let base = sdf_block(x,y,0.75,0.9);
    let ch = name.chars().last().unwrap_or('X');
    match ch {
        'A' => base.max(-sdf_block(x, y+0.15, 0.3, 0.05)),
        'B' | '8' => base.max(-sdf_circle_cut(x+0.15,y+0.3,0.25)).max(-sdf_circle_cut(x+0.15,y-0.3,0.25)),
        '0' => sdf_ring(x,y,0.75,0.45),
        '1' => sdf_block(x-0.2,y,0.15,0.9),
        'C' => sdf_ring_open(x,y,0.75,0.45,0.5, true),
        'D' => base.max(-sdf_circle_cut(x+0.15, y,0.55)),
        _ => base,
    }
}

fn sdf_block(x:f32,y:f32,hw:f32,hh:f32)->f32{ let dx=(x.abs()-hw).max(0.0); let dy=(y.abs()-hh).max(0.0); -(dx.hypot(dy)+ (hw - x.abs()).min(hh - y.abs()).min(0.0)) }
fn sdf_circle_cut(x:f32,y:f32,r:f32)->f32{ r - (x*x + y*y).sqrt() }
fn sdf_ring(x:f32,y:f32,ro:f32,ri:f32)->f32{ let d=(x*x + y*y).sqrt(); (ro - d).min(d - ri) }
fn sdf_ring_open(x:f32,y:f32,ro:f32,ri:f32,gap:f32,left:bool)->f32{ let d=(x*x + y*y).sqrt(); let mut sd=(ro - d).min(d - ri); if left { if x < -gap { sd = -sd.abs(); } } else { if x>gap { sd = -sd.abs(); } } sd }
