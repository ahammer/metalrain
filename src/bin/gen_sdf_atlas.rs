use std::fs;
use std::path::PathBuf;

// Minimal stub that writes a 128x128 white square PNG + JSON schema for a single circle shape.
// This is a placeholder; real generation would rasterize a signed distance field.
fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from("assets/shapes");
    fs::create_dir_all(&out_dir)?;
    let png_path = out_dir.join("sdf_atlas.png");
    let json_path = out_dir.join("sdf_atlas.json");
    // Write trivial grayscale PNG 128x128 mid gray (distance 0.5) using image crate.
    let mut img = image::GrayImage::new(128, 128);
    for p in img.pixels_mut() { p.0[0] = 128; }
    img.save(&png_path)?;
    let json = serde_json::json!({
        "version":1,
        "distance_range":8.0,
        "tile_size":128,
        "atlas_width":128,
        "atlas_height":128,
        "channel_mode":"sdf_r8",
        "shapes":[{
            "name":"circle_filled",
            "index":0,
            "px": {"x":0,"y":0,"w":128,"h":128},
            "uv": {"u0":0.0,"v0":0.0,"u1":1.0,"v1":1.0},
            "pivot": {"x":0.5,"y":0.5},
            "advance_scale":1.0,
            "metadata":{}
        }]
    });
    fs::write(json_path, serde_json::to_string_pretty(&json)?)?;
    println!("Wrote stub SDF atlas (circle) to assets/shapes.");
    Ok(())
}
