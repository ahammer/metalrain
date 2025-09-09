use std::{fs, path::PathBuf};
use clap::Parser;
use anyhow::Result;
use image::GenericImageView;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(about="Inspect an SDF atlas PNG + JSON for basic statistics", version)]
struct Args {
    #[arg(long)] atlas_png: PathBuf,
    #[arg(long)] atlas_json: PathBuf,
}

#[derive(Deserialize)]
struct ShapeEntry { name:String, index:u32, px:Rect, #[serde(default)] metadata: Option<serde_json::Value> }
#[derive(Deserialize)]
struct Rect { x:u32, y:u32, w:u32, h:u32 }
#[derive(Deserialize)]
struct AtlasRoot { version:u32, distance_range:f32, tile_size:u32, atlas_width:u32, atlas_height:u32, channel_mode:String, shapes:Vec<ShapeEntry> }

fn main() -> Result<()> {
    let args = Args::parse();
    let img = image::open(&args.atlas_png)?;
    let (w,h) = img.dimensions();
    let json_txt = fs::read_to_string(&args.atlas_json)?;
    let root: AtlasRoot = serde_json::from_str(&json_txt)?;
    println!("Atlas: {}x{} tilesize={} shapes={} mode={} dist_range={}", w,h, root.tile_size, root.shapes.len(), root.channel_mode, root.distance_range);

    // Global stats
    let mut hist = [0u64; 256];
    for p in img.to_luma8().pixels() { hist[p[0] as usize] += 1; }
    let total: u64 = hist.iter().sum();
    let mean: f64 = hist.iter().enumerate().map(|(i,c)| (i as f64)*( *c as f64)).sum::<f64>() / total as f64;
    let p128 = hist[128] as f64 / total as f64 * 100.0;
    let black = (hist[0]) as f64 / total as f64 * 100.0;
    let white = (hist[255]) as f64 / total as f64 * 100.0;
    println!("Global: mean={mean:.2} 128%={p128:.2}% black%={black:.2}% white%={white:.2}%");

    // Per-shape center sample & interior ratio estimate (count >128)
    let luma = img.to_luma8();
    for s in &root.shapes {
        let cx = s.px.x + s.px.w/2; let cy = s.px.y + s.px.h/2;
        if cx < w && cy < h { let v = luma.get_pixel(cx, cy)[0];
            // interior ratio
            let mut inside = 0u32; let mut total_px = 0u32;
            for yy in s.px.y..s.px.y + s.px.h { for xx in s.px.x..s.px.x + s.px.w { total_px += 1; if luma.get_pixel(xx,yy)[0] > 128 { inside += 1; } } }
            let ratio = inside as f64 / total_px as f64 * 100.0;
            let padding = s.metadata.as_ref().and_then(|m| m.get("padding_px")).and_then(|v| v.as_u64());
            if let Some(p) = padding { println!("Shape {:>2} {:<12} center={} inside>{}: {:.1}% padding_px={}", s.index, s.name, v, 128, ratio, p); }
            else { println!("Shape {:>2} {:<12} center={} inside>{}: {:.1}%", s.index, s.name, v, 128, ratio); }
        }
    }
    Ok(())
}
