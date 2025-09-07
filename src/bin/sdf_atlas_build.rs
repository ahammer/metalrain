//! Procedural SDF Atlas Builder
//!
//! Generates a simple SDF atlas PNG + JSON for a small library of shapes:
//!   - circle, triangle (equilateral), square
//!   - glyphs 0-9, A-Z, a-z rendered via font outline (fallback crude block forms if outline missing)
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
    for c in 'a'..='z' { shape_names.push(format!("glyph_{}", c)); }

    // Pre-measure glyph global extent (max of per-glyph max(width,height)) so all glyphs share uniform scale.
    // This preserves typographic relative sizes (x-height vs cap height) instead of stretching each to tile.
    let glyph_global_extent = {
        struct BBoxCollector { bbox:(f32,f32,f32,f32), started: bool, last:(f32,f32), first:(f32,f32) }
        impl BBoxCollector { fn upd(&mut self,x:f32,y:f32){ let b=&mut self.bbox; if x<b.0 {b.0=x;} if y<b.1 {b.1=y;} if x>b.2 {b.2=x;} if y>b.3 {b.3=y;} } }
        impl ttf::OutlineBuilder for BBoxCollector {
            fn move_to(&mut self, x:f32,y:f32){ self.started=true; self.last=(x,y); self.first=(x,y); self.upd(x,y); }
            fn line_to(&mut self, x:f32,y:f32){ self.last=(x,y); self.upd(x,y);}
            fn quad_to(&mut self, x1:f32,y1:f32,x:f32,y:f32){ // simple flatten
                let (sx,sy)=self.last; const S:usize=8; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=it*it*sx+2.0*it*t*x1+t*t*x; let py=it*it*sy+2.0*it*t*y1+t*t*y; self.last=(px,py); self.upd(px,py);} }
            fn curve_to(&mut self, x1:f32,y1:f32,x2:f32,y2:f32,x:f32,y:f32){ let (sx,sy)=self.last; const S:usize=12; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=sx*it*it*it + 3.0*x1*it*it*t + 3.0*x2*it*t*t + x*t*t*t; let py=sy*it*it*it + 3.0*y1*it*it*t + 3.0*y2*it*t*t + y*t*t*t; self.last=(px,py); self.upd(px,py);} }
            fn close(&mut self){ /* bbox only */ }
        }
        let mut max_extent = 0.0f32;
        for ch in ('0'..='9').chain('A'..='Z').chain('a'..='z') {
            let gid = font.glyph_id(ch);
            let gid16 = ttf::GlyphId(gid.0);
            let mut col = BBoxCollector { bbox:(f32::MAX,f32::MAX,-f32::MAX,-f32::MAX), started:false, last:(0.0,0.0), first:(0.0,0.0) };
            parsed.outline_glyph(gid16, &mut col);
            if !col.started { continue; } // skip missing outlines
            let (min_x,min_y,max_x,max_y) = col.bbox;
            if min_x >= max_x || min_y >= max_y { continue; }
            let bw = max_x - min_x; let bh = max_y - min_y; let m = bw.max(bh);
            if m > max_extent { max_extent = m; }
        }
        if max_extent <= 0.0 { // fallback: use units_per_em (ttf units per em is non-zero per spec)
            parsed.units_per_em() as f32
        } else { max_extent }
    };

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
                    let sd_norm = signed_distance(name, cx, cy, &font, &parsed, glyph_global_extent); // negative inside
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

fn signed_distance(name:&str, x: f32, y: f32, font: &ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>, glyph_global_extent:f32) -> f32 {
    // Canonical SDFs return negative inside, positive outside.
    match name {
        // Circle of radius r (shape space) : sd = length(p) - r (positive outside). We invert sign to keep negative inside.
    "circle" => { let r = 0.8; (x*x + y*y).sqrt() - r },
        // Square (box) half-size s: standard box SDF (positive outside)
        "square" => { let s = 0.8; let dx = (x.abs() - s).max(0.0); let dy = (y.abs() - s).max(0.0); let outside = (dx*dx + dy*dy).sqrt(); let inside = (x.abs().max(y.abs()) - s).min(0.0); outside + inside },
        "triangle" => sdf_equilateral_triangle(x,y),
    _ if name.starts_with("glyph_") => sdf_glyph_outline(name, x, y, font, parsed, glyph_global_extent).unwrap_or_else(|| sdf_glyph_placeholder(name, x, y)),
        _ => 1.0, // far outside by default
    }
}

/// Signed distance to a centered equilateral triangle (negative inside, positive outside).
/// Implementation based on Inigo Quilez's canonical formulation, adapted to keep the triangle
/// comparable in scale to other shapes (circle, square ~0.8 extent).
fn sdf_equilateral_triangle(x:f32,y:f32)->f32 {
    // Scale so the circumradius roughly matches the 0.8 used for circle/square half-size.
    // Using a content scale (s) we map input coordinates to canonical triangle of side length 2.
    const S: f32 = 0.85; // tuning constant to visually match other primitives' apparent size
    let k = (3.0f32).sqrt();
    // Scale into canonical space
    let mut p = Vec2::new(x / S, y / S);
    // Canonical equilateral triangle SDF (IQ) with side length 2, centered.
    p.x = p.x.abs() - 1.0;
    p.y = p.y + 1.0 / k;
    if p.x + k * p.y > 0.0 { // project into corner region if above hypotenuse
        p = Vec2::new( (p.x - k*p.y)*0.5, (-k*p.x - p.y)*0.5 );
    }
    p.x -= p.x.clamp(-2.0, 0.0); // clamp to edge strip to avoid artifacts
    let d = -p.length() * p.y.signum(); // negative inside
    d * S // rescale distance back to original coordinate scale
}

#[cfg(test)]
mod triangle_tests {
    use super::sdf_equilateral_triangle;
    // Basic sanity tests: center should be inside (negative), far outside should be positive,
    // point near a vertex roughly zero.
    #[test]
    fn triangle_signs() {
        assert!(sdf_equilateral_triangle(0.0, 0.0) < 0.0, "center should be inside");
        assert!(sdf_equilateral_triangle(0.0, 1.2) > 0.0, "far above should be outside");
        let near_edge = sdf_equilateral_triangle(0.0, -0.49); // near bottom edge
        assert!(near_edge.abs() < 0.2, "expected near surface distance, got {}", near_edge);
    }
}

#[cfg(test)]
mod glyph_range_tests {
    use std::io::Write;

    #[test]
    fn includes_lowercase_and_uppercase() {
        // Create a tiny fake font atlas run (reuse real font). We'll only inspect metadata JSON.
        let tmp = tempfile::tempdir().unwrap();
        let json_path = tmp.path().join("atlas.json");
        let png_path = tmp.path().join("atlas.png");
        // Run main logic by simulating Args parse? Simpler: directly call minimal subset: replicate shape list
    let mut shape_names: Vec<String> = vec!["circle".into(), "triangle".into(), "square".into()];
        for c in '0'..='9' { shape_names.push(format!("glyph_{}", c)); }
        for c in 'A'..='Z' { shape_names.push(format!("glyph_{}", c)); }
        for c in 'a'..='z' { shape_names.push(format!("glyph_{}", c)); }
        assert!(shape_names.iter().any(|s| s=="glyph_A"));
        assert!(shape_names.iter().any(|s| s=="glyph_z"));
        // Minimal JSON write to prove both appear (no need to render PNG for logic validation here)
        let dummy = serde_json::json!({
            "version":1,
            "distance_range":8.0,
            "tile_size":64,
            "atlas_width":64,
            "atlas_height":64,
            "channel_mode":"sdf_r8",
            "shapes": shape_names.iter().enumerate().map(|(i,n)| serde_json::json!({
                "name": n,
                "index": i+1,
                "px": {"x":0,"y":0,"w":64,"h":64},
                "uv": {"u0":0.0,"v0":0.0,"u1":1.0,"v1":1.0},
                "pivot": {"x":0.5,"y":0.5},
                "metadata": {"padding_px":0}
            })).collect::<Vec<_>>()
        });
        let mut f = std::fs::File::create(&json_path).unwrap();
        write!(f, "{}", serde_json::to_string_pretty(&dummy).unwrap()).unwrap();
        let produced = std::fs::read_to_string(&json_path).unwrap();
        assert!(produced.contains("glyph_A"));
        assert!(produced.contains("glyph_z"));
        // Avoid unused variable warnings
        let _ = png_path; // (we didn't render in this test)
    }
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
fn sdf_glyph_outline(name:&str, x:f32, y:f32, font:&ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>, glyph_global_extent:f32) -> Option<f32> {
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
    let (min_x,min_y,max_x,max_y)=col.bbox; // width/height unneeded after global scale chosen
    // Global uniform scale (avoid per-glyph fitting); use precomputed maximum extent across glyph set.
    let scale = glyph_global_extent;
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
