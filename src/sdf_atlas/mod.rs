//! Unified SDF atlas generation & inspection module.
//! Extracted from former standalone binaries.

use anyhow::{Result, bail};
use serde::{Serialize, Deserialize};
use std::{fs, path::{Path, PathBuf}};
use image::GrayImage;
use ab_glyph::Font;
use bevy::prelude::Vec2;
use ttf_parser as ttf;

#[derive(Clone, Debug)]
pub struct BuildConfig {
    pub tile_size: u32,
    pub padding_px: u32,
    pub distance_span_factor: f32,
    pub channel_mode: String,
    pub out_stem: PathBuf,
    pub out_png: Option<PathBuf>,
    pub out_json: Option<PathBuf>,
    pub json_only: bool,
    pub png_only: bool,
    pub stdout_json: bool,
    pub overwrite: bool,
    pub font_path: PathBuf,
    pub supersamples: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutPivot { pub x:f32, pub y:f32 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutUv { pub u0:f32, pub v0:f32, pub u1:f32, pub v1:f32 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RectPx { pub x:u32, pub y:u32, pub w:u32, pub h:u32 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutShapeEntry { pub name:String, pub index:u32, pub px:RectPx, pub uv:OutUv, pub pivot:OutPivot, #[serde(skip_serializing_if="Option::is_none")] pub metadata: Option<serde_json::Value> }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutRoot { pub version:u32, pub distance_range:f32, pub tile_size:u32, pub atlas_width:u32, pub atlas_height:u32, pub channel_mode:String, pub shapes:Vec<OutShapeEntry> }

pub struct AtlasArtifact { pub image: Option<GrayImage>, pub json: OutRoot, pub png_path: PathBuf, pub json_path: PathBuf }

pub struct Inspection { pub tile_size:u32, pub distance_range:f32, pub shape_count:usize, pub channel_mode:String, pub atlas_dim:(u32,u32) }

pub fn derive_output_paths(cfg:&BuildConfig) -> Result<(PathBuf,PathBuf)> {
    if let (Some(png), Some(json)) = (&cfg.out_png, &cfg.out_json) { return Ok((png.clone(), json.clone())); }
    let stem = if !cfg.out_stem.as_os_str().is_empty() { cfg.out_stem.clone() } else { PathBuf::from("assets/shapes/sdf_atlas") };
    Ok((cfg.out_png.clone().unwrap_or_else(|| stem.with_extension("png")), cfg.out_json.clone().unwrap_or_else(|| stem.with_extension("json"))))
}

pub fn build_atlas(cfg:&BuildConfig) -> Result<AtlasArtifact> {
    if cfg.json_only && cfg.png_only { bail!("--json-only and --png-only conflict"); }
    if cfg.padding_px*2 >= cfg.tile_size { bail!("padding too large"); }
    if !matches!(cfg.channel_mode.as_str(), "sdf_r8"|"msdf_rgb"|"msdf_rgba"|"single") { bail!("unsupported channel_mode {}", cfg.channel_mode); }
    let (png_path, json_path) = derive_output_paths(cfg)?;
    if !cfg.overwrite { for p in [&png_path,&json_path] { if p.exists() { bail!("Refusing to overwrite {} (use --overwrite)", p.display()); } } }

    // Prepare font
    let font_bytes = fs::read(&cfg.font_path).map_err(|e| anyhow::anyhow!("read font {:?}: {e}", cfg.font_path))?;
    let font = ab_glyph::FontRef::try_from_slice(&font_bytes)?;
    let parsed = ttf::Face::parse(&font_bytes, 0).map_err(|_| anyhow::anyhow!("ttf parse failed"))?;

    let shape_names = shape_name_list();
    let count = shape_names.len() as u32;
    let cols = 8u32; // keep simple deterministic packing
    let rows = ((count + cols - 1)/cols).max(1);
    let atlas_w = cols * cfg.tile_size; let atlas_h = rows * cfg.tile_size;
    let distance_range_px = (cfg.tile_size as f32 * cfg.distance_span_factor).max(1.0);
    let mut img = if cfg.json_only { None } else { Some(GrayImage::from_pixel(atlas_w, atlas_h, image::Luma([128]))) };

    // supersample pattern
    let ss = if cfg.supersamples >=4 {4} else if cfg.supersamples>=2 {4} else {1};
    let offsets: &[(f32,f32)] = if ss==1 { &[(0.5,0.5)] } else { &[(0.25,0.25),(0.75,0.25),(0.25,0.75),(0.75,0.75)] };

    // Global glyph extent
    let glyph_global_extent = measure_global_extent(&parsed, &font);

    let inner_size = (cfg.tile_size - cfg.padding_px*2) as f32;
    for (idx,name) in shape_names.iter().enumerate() { if let Some(at) = img.as_mut() {
        let ix = idx as u32; let col = ix % cols; let row = ix / cols; let ox = col * cfg.tile_size; let oy = row * cfg.tile_size; let tile_i = cfg.tile_size as i32;
        for py in 0..tile_i { for px in 0..tile_i { let mut accum=0.0; for (oxs,oys) in offsets { let fx_c = ((px as f32+oxs) - cfg.padding_px as f32)/inner_size; let fy_c = ((py as f32+oys) - cfg.padding_px as f32)/inner_size; let cx = fx_c*2.0 - 1.0; let cy = fy_c*2.0 - 1.0; let sd_norm = signed_distance(name, cx, cy, &font, &parsed, glyph_global_extent); let sd_px = sd_norm * (cfg.tile_size as f32 * 0.5); let sd_clamped = sd_px.clamp(-distance_range_px, distance_range_px); let n = (0.5 - sd_clamped / distance_range_px).clamp(0.0,1.0); accum += n; } let avg = accum/(ss as f32); let v = (avg*255.0).round().clamp(0.0,255.0) as u8; at.put_pixel(ox+px as u32, oy+py as u32, image::Luma([v])); } }
    }}

    // Metadata construction
    let mut shapes_meta = Vec::new();
    for (i,name) in shape_names.iter().enumerate() {
        let ix = i as u32; let col = ix % cols; let row = ix / cols; let x = col * cfg.tile_size; let y = row * cfg.tile_size;
        let uv0 = x as f32 / atlas_w as f32; let uv1 = (x + cfg.tile_size) as f32 / atlas_w as f32; let v0 = y as f32 / atlas_h as f32; let v1 = (y + cfg.tile_size) as f32 / atlas_h as f32;
        shapes_meta.push(OutShapeEntry { name: name.clone(), index: ix+1, px: RectPx { x, y, w: cfg.tile_size, h: cfg.tile_size }, uv: OutUv { u0: uv0, v0, u1: uv1, v1 }, pivot: OutPivot { x:0.5, y:0.5 }, metadata: Some(serde_json::json!({"padding_px": cfg.padding_px})) });
    }
    let root = OutRoot { version:1, distance_range: distance_range_px, tile_size: cfg.tile_size, atlas_width: atlas_w, atlas_height: atlas_h, channel_mode: cfg.channel_mode.clone(), shapes: shapes_meta };
    Ok(AtlasArtifact { image: img, json: root, png_path, json_path })
}

pub fn write_outputs(artifact:&AtlasArtifact, cfg:&BuildConfig) -> Result<()> {
    if let Some(parent) = artifact.png_path.parent() { fs::create_dir_all(parent)?; }
    if let Some(parent) = artifact.json_path.parent() { fs::create_dir_all(parent)?; }
    if !cfg.json_only { if let Some(ref img) = artifact.image { img.save(&artifact.png_path)?; } }
    if !cfg.png_only { let js = serde_json::to_string_pretty(&artifact.json)?; fs::write(&artifact.json_path, js)?; if cfg.stdout_json { println!("{}", serde_json::to_string_pretty(&artifact.json)?); } }
    Ok(())
}

pub fn inspect(json_path:&Path, maybe_png:Option<&Path>) -> Result<Inspection> {
    let txt = fs::read_to_string(json_path)?; let root: OutRoot = serde_json::from_str(&txt)?;
    if let Some(p) = maybe_png { if p.exists() { let img = image::open(p)?; if img.width()!=root.atlas_width || img.height()!=root.atlas_height { eprintln!("warning: png size mismatch"); } } }
    Ok(Inspection { tile_size: root.tile_size, distance_range: root.distance_range, shape_count: root.shapes.len(), channel_mode: root.channel_mode, atlas_dim:(root.atlas_width, root.atlas_height) })
}

fn shape_name_list() -> Vec<String> {
    let mut v = vec!["circle".into(), "triangle".into(), "square".into()];
    for c in '0'..='9' { v.push(format!("glyph_{}", c)); }
    for c in 'A'..='Z' { v.push(format!("glyph_{}", c)); }
    for c in 'a'..='z' { v.push(format!("glyph_{}", c)); }
    v
}

fn measure_global_extent(parsed:&ttf::Face<'_>, font:&ab_glyph::FontRef<'_>) -> f32 {
    let mut max_extent=0.0f32;
    for ch in ('0'..='9').chain('A'..='Z').chain('a'..='z') { let gid = font.glyph_id(ch); let mut col = GlyphBBox { bbox:(f32::MAX,f32::MAX,-f32::MAX,-f32::MAX), started:false, last:(0.0,0.0) }; parsed.outline_glyph(ttf::GlyphId(gid.0), &mut col); if !col.started { continue; } let (min_x,min_y,max_x,max_y)=col.bbox; let bw = max_x-min_x; let bh = max_y-min_y; let m = bw.max(bh); if m>max_extent { max_extent=m; } }
    if max_extent <= 0.0 { parsed.units_per_em() as f32 } else { max_extent }
}

struct GlyphBBox { bbox:(f32,f32,f32,f32), started: bool, last:(f32,f32) }
impl GlyphBBox { fn upd(&mut self,x:f32,y:f32){ let b=&mut self.bbox; if x<b.0 {b.0=x;} if y<b.1 {b.1=y;} if x>b.2 {b.2=x;} if y>b.3 {b.3=y;} } }
impl ttf::OutlineBuilder for GlyphBBox {
    fn move_to(&mut self, x:f32,y:f32){ self.started=true; self.last=(x,y); self.upd(x,y); }
    fn line_to(&mut self, x:f32,y:f32){ self.last=(x,y); self.upd(x,y); }
    fn quad_to(&mut self, x1:f32,y1:f32,x:f32,y:f32){ let (sx,sy)=self.last; const S:usize=8; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=it*it*sx+2.0*it*t*x1+t*t*x; let py=it*it*sy+2.0*it*t*y1+t*t*y; self.last=(px,py); self.upd(px,py);} }
    fn curve_to(&mut self, x1:f32,y1:f32,x2:f32,y2:f32,x:f32,y:f32){ let (sx,sy)=self.last; const S:usize=12; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=sx*it*it*it+3.0*x1*it*it*t+3.0*x2*it*t*t+x*t*t*t; let py=sy*it*it*it+3.0*y1*it*it*t+3.0*y2*it*t*t+y*t*t*t; self.last=(px,py); self.upd(px,py);} }
    fn close(&mut self){}
}

fn signed_distance(name:&str, x:f32, y:f32, font:&ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>, glyph_global_extent:f32) -> f32 {
    match name {
        "circle" => { let r=0.8; (x*x + y*y).sqrt() - r },
        "square" => { let s=0.8; let dx=(x.abs()-s).max(0.0); let dy=(y.abs()-s).max(0.0); let outside=(dx*dx+dy*dy).sqrt(); let inside=(x.abs().max(y.abs())-s).min(0.0); outside+inside },
        "triangle" => sdf_equilateral_triangle(x,y),
        _ if name.starts_with("glyph_") => sdf_glyph_outline(name,x,y,font,parsed,glyph_global_extent).unwrap_or_else(|| 1.0),
        _ => 1.0,
    }
}

fn sdf_equilateral_triangle(x:f32,y:f32)->f32 { const S:f32=0.85; let k=(3.0f32).sqrt(); let mut p=Vec2::new(x/S,y/S); p.x=p.x.abs()-1.0; p.y=p.y + 1.0/k; if p.x + k*p.y > 0.0 { p = Vec2::new((p.x - k*p.y)*0.5, (-k*p.x - p.y)*0.5); } p.x -= p.x.clamp(-2.0,0.0); let d = -p.length()*p.y.signum(); d*S }

fn sdf_glyph_outline(name:&str, x:f32, y:f32, font:&ab_glyph::FontRef<'_>, parsed:&ttf::Face<'_>, glyph_global_extent:f32) -> Option<f32> {
    let ch = name.strip_prefix("glyph_")?.chars().next()?; let gid = font.glyph_id(ch); let gid16 = ttf::GlyphId(gid.0);
    struct Collector { segs:Vec<((f32,f32),(f32,f32))>, last:(f32,f32), start:(f32,f32), bbox:(f32,f32,f32,f32), started:bool }
    impl Collector { fn upd(&mut self,x:f32,y:f32){ let b=&mut self.bbox; if x<b.0 {b.0=x;} if y<b.1 {b.1=y;} if x>b.2 {b.2=x;} if y>b.3 {b.3=y;} } fn start(&mut self,x:f32,y:f32){ self.started=true; self.last=(x,y); self.start=(x,y); self.upd(x,y);} fn close(&mut self){ if self.started && (self.last!=self.start) { let s=self.last; let e=self.start; self.segs.push((s,e)); } }}
    impl ttf::OutlineBuilder for Collector { fn move_to(&mut self,x:f32,y:f32){ self.close(); self.start(x,y);} fn line_to(&mut self,x:f32,y:f32){ let s=self.last; self.segs.push((s,(x,y))); self.last=(x,y); self.upd(x,y);} fn quad_to(&mut self,x1:f32,y1:f32,x:f32,y:f32){ let (sx,sy)=self.last; const S:usize=12; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=it*it*sx+2.0*it*t*x1+t*t*x; let py=it*it*sy+2.0*it*t*y1+t*t*y; self.segs.push((self.last,(px,py))); self.last=(px,py); self.upd(px,py);} } fn curve_to(&mut self,x1:f32,y1:f32,x2:f32,y2:f32,x:f32,y:f32){ let (sx,sy)=self.last; const S:usize=18; for i in 1..=S { let t=i as f32/S as f32; let it=1.0-t; let px=sx*it*it*it+3.0*x1*it*it*t+3.0*x2*it*t*t+x*t*t*t; let py=sy*it*it*it+3.0*y1*it*it*t+3.0*y2*it*t*t+y*t*t*t; self.segs.push((self.last,(px,py))); self.last=(px,py); self.upd(px,py);} } fn close(&mut self){ self.close(); } }
    let mut col = Collector { segs:Vec::new(), last:(0.0,0.0), start:(0.0,0.0), bbox:(f32::MAX,f32::MAX,-f32::MAX,-f32::MAX), started:false }; parsed.outline_glyph(gid16,&mut col); if col.segs.is_empty() { return None; }
    let (min_x,min_y,max_x,max_y)=col.bbox; let cx = (min_x+max_x)*0.5; let cy=(min_y+max_y)*0.5; let scale = glyph_global_extent; let fx = cx + x*(scale*0.5); let fy = cy + (-y)*(scale*0.5);
    let mut parity=false; for (a,b) in &col.segs { let (ax,ay)=*a; let (bx,by)=*b; if ((ay>fy)!=(by>fy)) && (fx < (bx-ax)*(fy-ay)/(by-ay+1e-6)+ax) { parity=!parity; } }
    let inside = parity; let mut min_d2=f32::MAX; for (a,b) in &col.segs { let (ax,ay)=a; let (bx,by)=b; let vx=bx-ax; let vy=by-ay; let wx=fx-ax; let wy=fy-ay; let ll=vx*vx+vy*vy; let t = if ll<=1e-6 {0.0} else {(vx*wx+vy*wy)/ll}; let tt=t.clamp(0.0,1.0); let px=ax+vx*tt; let py=ay+vy*tt; let dx=fx-px; let dy=fy-py; let d2=dx*dx+dy*dy; if d2<min_d2 { min_d2=d2; } }
    let dist = min_d2.sqrt(); let norm_half=0.5*scale; let sd = dist / norm_half; Some(if inside { -sd } else { sd })
}

#[cfg(test)]
mod tests {
    #[test]
    fn triangle_sign() { assert!(super::sdf_equilateral_triangle(0.0,0.0) < 0.0); }

    #[test]
    fn glyph_catalog_includes_cases() {
        let names = super::shape_name_list();
        assert!(names.iter().any(|s| s=="glyph_A"));
        assert!(names.iter().any(|s| s=="glyph_z"));
        assert!(names.iter().any(|s| s=="glyph_0"));
        // Ensure no duplicates
        let mut sorted = names.clone();
        sorted.sort();
        for w in sorted.windows(2) { assert!(w[0]!=w[1], "duplicate shape name {}", w[0]); }
    }
}
