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
use ab_glyph::Font; // trait for glyph_id
use ttf_parser as ttf;
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
    #[arg(long, default_value_t = 0.5)] distance_span_factor: f32, // distance_range_px = tile_size * factor (default widened for better gradient)
    #[arg(long, default_value_t = 1)] supersamples: u32, // 1, 2, or 4 (grid 1x1, 2x2)
    /// TrueType / OpenType font path for alphanumeric glyphs (0-9, A-Z). Defaults to bundled DroidSansMono.
    #[arg(long, default_value = "assets/fonts/DroidSansMono.ttf")] font: String,
    /// Padding in pixels reserved around each shape/glyph inside its tile (applies to all entries).
    /// Ensures the signed distance field can fully transition to background (black) without clipping.
    #[arg(long, default_value_t = 0)] padding_px: u32,
}

#[derive(Serialize)] struct OutPivot { x: f32, y: f32 }
#[derive(Serialize)] struct OutUv { u0: f32, v0: f32, u1: f32, v1: f32 }
#[derive(Serialize)] struct OutRect { x: u32, y: u32, w: u32, h: u32 }
#[derive(Serialize)] struct OutShapeEntry { name:String, index:u32, px:OutRect, uv:OutUv, pivot:OutPivot, #[serde(skip_serializing_if="Option::is_none")] advance_scale: Option<f32>, metadata: serde_json::Value }
#[derive(Serialize)] struct OutRoot { version:u32, distance_range:f32, tile_size:u32, atlas_width:u32, atlas_height:u32, channel_mode:String, shapes:Vec<OutShapeEntry> }

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.tile_size < 16 { anyhow::bail!("tile_size too small (min 16)"); }
    if !matches!(args.channel_mode.as_str(), "sdf_r8"|"msdf_rgb"|"msdf_rgba") { anyhow::bail!("unsupported channel_mode {}",&args.channel_mode); }

    // Load font for glyph outlines (ab_glyph for glyph ids; ttf-parser for precise outlines)
    let font_data = fs::read(&args.font).map_err(|e| anyhow::anyhow!("read font {}: {e}", args.font))?;
    let font = ab_glyph::FontRef::try_from_slice(&font_data).map_err(|e| anyhow::anyhow!("decode font: {e}"))?;
    let parsed = ttf::Face::parse(&font_data, 0).map_err(|_| anyhow::anyhow!("ttf parse failed"))?;

    // Shape catalog order determines index assignment (start at 1)
    let mut shape_names: Vec<String> = vec!["circle".into(), "triangle".into(), "square".into()];
    for c in '0'..='9' { shape_names.push(format!("glyph_{}", c)); }
    for c in 'A'..='Z' { shape_names.push(format!("glyph_{}", c)); }

    let tile = args.tile_size as i32;
    if args.padding_px * 2 >= args.tile_size {
        anyhow::bail!("padding_px*2 >= tile_size ({} * 2 >= {})", args.padding_px, args.tile_size);
    }
    let inner_size = (args.tile_size - args.padding_px * 2) as f32; // drawable logical dimension per axis for content
    let grid_cols = 8i32; // adjustable
    let grid_rows = ((shape_names.len() as i32 + grid_cols - 1) / grid_cols).max(1);
    let atlas_w = (grid_cols as u32) * args.tile_size;
    let atlas_h = (grid_rows as u32) * args.tile_size;

    let mut atlas: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_pixel(atlas_w, atlas_h, Luma([128u8])); // 0 distance neutral

    // distance_range in *pixels* that runtime will use when decoding back to a signed distance.
    let distance_range_px = (args.tile_size as f32 * args.distance_span_factor).max(1.0);
    // Precompute supersample pattern (supports 1 or 4 samples; 2 -> treat as 4 for simplicity)
    let ss = if args.supersamples >= 4 { 4 } else if args.supersamples >= 2 { 4 } else { 1 };
    let offsets: &[(f32,f32)] = if ss == 1 { &[(0.5,0.5)] } else { &[(0.25,0.25),(0.75,0.25),(0.25,0.75),(0.75,0.75)] };

    for (idx, name) in shape_names.iter().enumerate() {
        let col = (idx as i32) % grid_cols;
        let row = (idx as i32) / grid_cols;
        let ox = col * tile;
        let oy = row * tile;
        for py in 0..tile {
            for px in 0..tile {
                let mut accum = 0.0f32;
                for (oxs, oys) in offsets {
                    // Normalized content space inside padding (0..1). Pixels inside padding map <0 or >1 -> treated as outside shape region.
                    let fx_content = ((px as f32 + oxs) - args.padding_px as f32) / inner_size;
                    let fy_content = ((py as f32 + oys) - args.padding_px as f32) / inner_size;
                    // Canonical shape space [-1,1] over just the content region (avoids warping by guaranteeing square mapping).
                    let cx = fx_content * 2.0 - 1.0;
                    let cy = fy_content * 2.0 - 1.0;
                    // Raw normalized signed distance (negative inside, positive outside after normalization step below)
                    let sd_norm = signed_distance(name, cx, cy, &font, &parsed); // negative inside
                    let sd_px = sd_norm * (args.tile_size as f32 * 0.5);
                    let sd_clamped = sd_px.clamp(-distance_range_px, distance_range_px);
                    // Inverted encoding: inside (sd < 0) becomes brighter than 0.5 ( > 128 ),
                    // outside becomes darker (< 128). This matches desired visual of white interior, black exterior.
                    let n = (0.5 - sd_clamped / distance_range_px).clamp(0.0,1.0); // 0.5 center surface
                    accum += n;
                }
                let n_avg = accum / (ss as f32);
                let u8v = (n_avg * 255.0).round().clamp(0.0,255.0) as u8;
                atlas.put_pixel((ox + px) as u32, (oy + py) as u32, Luma([u8v]));
            }
        }
    }

    if let Some(parent) = args.out_png.parent() { fs::create_dir_all(parent)?; }
    atlas.save(&args.out_png)?;

    // Build JSON metadata entries
    // TODO: capture advance width for glyph entries if text layout in atlas space becomes needed.
    let mut shapes_meta = Vec::new();
    for (i,name) in shape_names.iter().enumerate() {
        let col = (i as i32) % grid_cols; let row = (i as i32) / grid_cols; let x = (col * tile) as u32; let y = (row * tile) as u32; let uv0 = x as f32 / atlas_w as f32; let uv1 = (x + args.tile_size) as f32 / atlas_w as f32; let v0 = y as f32 / atlas_h as f32; let v1 = (y + args.tile_size) as f32 / atlas_h as f32;
        let meta = serde_json::json!({
            "padding_px": args.padding_px
        });
        shapes_meta.push(OutShapeEntry { name: name.clone(), index: (i+1) as u32, px: OutRect { x, y, w: args.tile_size, h: args.tile_size }, uv: OutUv { u0: uv0, v0, u1: uv1, v1 }, pivot: OutPivot { x:0.5, y:0.5 }, advance_scale: None, metadata: meta });
    }
    let root = OutRoot { version:1, distance_range: distance_range_px, tile_size: args.tile_size, atlas_width: atlas_w, atlas_height: atlas_h, channel_mode: args.channel_mode, shapes: shapes_meta };
    let json_str = serde_json::to_string_pretty(&root)?; if let Some(p) = args.out_json.parent() { fs::create_dir_all(p)?; } fs::write(&args.out_json, json_str)?;
    println!("Generated procedural SDF atlas: {} ({}x{})", args.out_png.display(), atlas_w, atlas_h);
    Ok(())
}

fn signed_distance(name:&str, x: f32, y: f32, font: &ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>) -> f32 {
    // Canonical SDFs return negative inside, positive outside.
    match name {
        // Circle of radius r (shape space) : sd = length(p) - r (positive outside). We invert sign to keep negative inside.
    "circle" => { let r = 0.8; (x*x + y*y).sqrt() - r },
        // Square (box) half-size s: standard box SDF (positive outside)
        "square" => { let s = 0.8; let dx = (x.abs() - s).max(0.0); let dy = (y.abs() - s).max(0.0); let outside = (dx*dx + dy*dy).sqrt(); let inside = (x.abs().max(y.abs()) - s).min(0.0); outside + inside },
        "triangle" => sdf_equilateral_triangle(x,y),
    _ if name.starts_with("glyph_") => sdf_glyph_outline(name, x, y, font, parsed).unwrap_or_else(|| sdf_glyph_placeholder(name, x, y)),
        _ => 1.0, // far outside by default
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

fn sdf_glyph_placeholder(name:&str, x:f32,y:f32)->f32{ // Fallback simple shape (only used if outline missing)
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

fn sdf_block(x:f32,y:f32,hw:f32,hh:f32)->f32{ // positive outside, negative inside (box SDF canonical)
    let dx = (x.abs() - hw).max(0.0);
    let dy = (y.abs() - hh).max(0.0);
    let outside = (dx*dx + dy*dy).sqrt();
    let inside = (x.abs().max(y.abs()) - hw.max(hh)).min(0.0);
    outside + inside
}
fn sdf_circle_cut(x:f32,y:f32,r:f32)->f32{ (x*x + y*y).sqrt() - r }
fn sdf_ring(x:f32,y:f32,ro:f32,ri:f32)->f32{ let d=(x*x + y*y).sqrt(); let outer = d - ro; let inner = ri - d; outer.max(inner) } // union of outside outer and inside inner
fn sdf_ring_open(x:f32,y:f32,ro:f32,ri:f32,gap:f32,left:bool)->f32{ let base = sdf_ring(x,y,ro,ri); if left { if x < -gap { base } else { base.max(x + gap) } } else { if x > gap { base } else { base.max(-x + gap) } } }

// Outline-based glyph SDF:
// 1. Extract glyph outline via ttf-parser (lines, quads, cubics -> flattened to line segments).
// 2. Compute signed distance using even-odd winding (parity) for inside test + min distance to segments.
// 3. Normalize by half of max dimension so outer bounds ~= distance 1.
fn sdf_glyph_outline(name:&str, x:f32, y:f32, font:&ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>) -> Option<f32> {
    let ch = name.strip_prefix("glyph_")?.chars().next()?;
    let gid = font.glyph_id(ch);
    let gid16 = ttf::GlyphId(gid.0);
    // Collect segments + bounds
    struct Collector {
        segs: Vec<((f32,f32),(f32,f32))>,
        last:(f32,f32),
        contour_start:(f32,f32),
        bbox:(f32,f32,f32,f32),
        started: bool
    }
    impl Collector {
        fn upd(&mut self,x:f32,y:f32){
            let b=&mut self.bbox;
            if x<b.0 {b.0=x;} if y<b.1 {b.1=y;} if x>b.2 {b.2=x;} if y>b.3 {b.3=y;}
        }
        fn begin_contour(&mut self,x:f32,y:f32){
            self.last=(x,y);
            self.contour_start=(x,y);
            self.started=true;
            self.upd(x,y);
        }
        fn close_contour(&mut self){
            // Explicitly close if last point not equal to start (avoid zero-length seg duplication)
            if self.started && (self.last.0 != self.contour_start.0 || self.last.1 != self.contour_start.1) {
                let s=self.last; let e=self.contour_start; self.segs.push((s,e));
            }
        }
    }
    impl ttf::OutlineBuilder for Collector {
        fn move_to(&mut self, x:f32,y:f32){
            // Starting a new contour: close previous one first
            self.close_contour();
            self.begin_contour(x,y);
        }
        fn line_to(&mut self, x:f32,y:f32){ let s=self.last; self.segs.push((s,(x,y))); self.last=(x,y); self.upd(x,y); }
        fn quad_to(&mut self, x1:f32,y1:f32,x:f32,y:f32){ // flatten quadratic
            let (sx,sy)=self.last; const S:usize=12; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=it*it*sx+2.0*it*t*x1+t*t*x; let py=it*it*sy+2.0*it*t*y1+t*t*y; self.segs.push((self.last,(px,py))); self.last=(px,py); self.upd(px,py); }
        }
        fn curve_to(&mut self, x1:f32,y1:f32,x2:f32,y2:f32,x:f32,y:f32){ // flatten cubic
            let (sx,sy)=self.last; const S:usize=18; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=sx*it*it*it + 3.0*x1*it*it*t + 3.0*x2*it*t*t + x*t*t*t; let py=sy*it*it*it + 3.0*y1*it*it*t + 3.0*y2*it*t*t + y*t*t*t; self.segs.push((self.last,(px,py))); self.last=(px,py); self.upd(px,py); }
        }
        fn close(&mut self){ self.close_contour(); }
    }
    let mut col = Collector { segs: Vec::new(), last:(0.0,0.0), contour_start:(0.0,0.0), bbox:(f32::MAX,f32::MAX,-f32::MAX,-f32::MAX), started:false };
    parsed.outline_glyph(gid16, &mut col);
    if col.segs.is_empty() { return None; }
    let (min_x,min_y,max_x,max_y)=col.bbox; let bw=(max_x-min_x).max(1.0); let bh=(max_y-min_y).max(1.0);
    // Uniform scale (avoid warping): choose max dimension; center glyph in remaining axis.
    let scale = bw.max(bh);
    let cx_g = (min_x + max_x) * 0.5;
    let cy_g = (min_y + max_y) * 0.5;
    // Map shape space [-1,1] uniformly into glyph space about center. Invert Y to correct orientation.
    let fx = cx_g + x * (scale * 0.5);
    let fy = cy_g + (-y) * (scale * 0.5);
    // Even-odd
    let mut parity=false; for (a,b) in &col.segs { let (ax,ay)=*a; let (bx,by)=*b; if ((ay>fy)!=(by>fy)) && (fx < (bx-ax)*(fy-ay)/(by-ay+1e-6)+ax) { parity = !parity; } }
    let inside=parity;
    // Min distance to segments
    let mut min_d2=f32::MAX; for (a,b) in &col.segs { let (ax,ay)=*a; let (bx,by)=*b; let vx=bx-ax; let vy=by-ay; let wx=fx-ax; let wy=fy-ay; let ll=vx*vx+vy*vy; let t = if ll<=1e-6 {0.0} else {(vx*wx+vy*wy)/ll}; let tt=t.clamp(0.0,1.0); let px=ax+vx*tt; let py=ay+vy*tt; let dx=fx-px; let dy=fy-py; let d2=dx*dx+dy*dy; if d2<min_d2 { min_d2=d2; } }
    let dist = min_d2.sqrt();
    let norm_half=0.5*scale;
    let sd = dist / norm_half;
    Some(if inside { -sd } else { sd })
}
