# Generate utility modules and documentation

# Utils module files
utils_mod_rs = '''pub mod image;
pub mod math;

pub use image::*;
pub use math::*;
'''

utils_image_rs = '''use anyhow::Result;
use image::{DynamicImage, GrayImage, RgbImage};

/// Image processing utilities for SDF generation
pub struct ImageUtils;

impl ImageUtils {
    /// Convert image to grayscale
    pub fn to_grayscale(image: &DynamicImage) -> GrayImage {
        image.to_luma8()
    }
    
    /// Create a binary mask from grayscale image
    pub fn create_mask(image: &GrayImage, threshold: u8) -> GrayImage {
        let (width, height) = image.dimensions();
        let mut mask = GrayImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y).0[0];
                let value = if pixel > threshold { 255 } else { 0 };
                mask.put_pixel(x, y, image::Luma([value]));
            }
        }
        
        mask
    }
    
    /// Resize image maintaining aspect ratio
    pub fn resize_with_padding(
        image: &DynamicImage,
        target_size: u32,
        background: [u8; 4],
    ) -> DynamicImage {
        let (width, height) = image.dimensions();
        let max_dim = width.max(height);
        let scale = target_size as f32 / max_dim as f32;
        
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        
        let resized = image.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        );
        
        // Center on background
        let mut result = image::RgbaImage::from_pixel(
            target_size,
            target_size,
            image::Rgba(background),
        );
        
        let offset_x = (target_size - new_width) / 2;
        let offset_y = (target_size - new_height) / 2;
        
        let resized_rgba = resized.to_rgba8();
        for y in 0..new_height {
            for x in 0..new_width {
                let pixel = resized_rgba.get_pixel(x, y);
                result.put_pixel(offset_x + x, offset_y + y, *pixel);
            }
        }
        
        DynamicImage::ImageRgba8(result)
    }
}
'''

utils_math_rs = '''/// Mathematical utilities for SDF generation
pub struct MathUtils;

impl MathUtils {
    /// Clamp value between min and max
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.max(min).min(max)
    }
    
    /// Linear interpolation between two values
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
    
    /// Smoothstep function for smooth interpolation
    pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = Self::clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
    
    /// Distance between two 2D points
    pub fn distance_2d(p1: (f32, f32), p2: (f32, f32)) -> f32 {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Normalize value from one range to another
    pub fn remap(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
        let t = (value - in_min) / (in_max - in_min);
        Self::lerp(out_min, out_max, t)
    }
}
'''

# Documentation files
readme_md = '''# Rust SDF Generator

A Rust application that generates signed distance fields (SDFs) for alphanumeric characters and basic geometric shapes, outputting sprite sheets and registry files optimized for shader usage.

## Features

- ✨ Generate SDFs for a-zA-Z0-9 characters from TTF/OTF fonts
- 🔶 Create SDFs for basic shapes (circle, triangle, square)
- 📦 Pack SDFs into efficient sprite sheets/texture atlases
- 📋 Generate JSON registry files with coordinate and metrics data
- 🎨 Support for both single-channel and multi-channel (MSDF) SDFs
- 🚀 Fast, memory-efficient pure Rust implementation
- ⚙️ Command-line interface with extensive customization options

## Installation

Ensure you have Rust installed, then clone and build:

```bash
git clone <repository-url>
cd sdf-generator
cargo build --release
```

## Usage

### Basic Usage

Generate SDFs from a font file:

```bash
cargo run -- --font-path ./fonts/MyFont.ttf --output-dir ./output
```

### Include Basic Shapes

```bash
cargo run -- --font-path ./fonts/MyFont.ttf --shapes --output-dir ./output
```

### Multi-channel SDF (MSDF)

For higher quality with sharp corners:

```bash
cargo run -- --font-path ./fonts/MyFont.ttf --multi-channel --output-dir ./output
```

### Custom Configuration

```bash
cargo run -- \\
  --font-path ./fonts/MyFont.ttf \\
  --output-dir ./output \\
  --sdf-size 128 \\
  --atlas-size 2048 \\
  --sdf-range 8 \\
  --padding 4 \\
  --shapes \\
  --characters "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%"
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--font-path` | Path to TTF/OTF font file | None |
| `--output-dir` | Output directory for files | `./output` |
| `--sdf-size` | SDF texture size per glyph | `64` |
| `--atlas-size` | Atlas dimensions (square) | `1024` |
| `--sdf-range` | Distance field range in pixels | `4.0` |
| `--padding` | Padding between glyphs | `2` |
| `--shapes` | Include basic shapes | `false` |
| `--multi-channel` | Use MSDF instead of single channel | `false` |
| `--characters` | Custom character set | `a-zA-Z0-9` |

## Output Files

### Sprite Sheet (`sdf_atlas.png`)

A packed texture atlas containing all generated SDFs with configurable padding and dimensions.

### Registry (`sdf_registry.json`)

JSON file containing:
- Atlas metadata (dimensions, format, creation date)
- Character metrics (position, size, bearing, advance)
- Shape information (position, size)

Example registry structure:

```json
{
  "metadata": {
    "atlas_width": 1024,
    "atlas_height": 1024,
    "sdf_range": 4.0,
    "resolution": 64,
    "padding": 2,
    "format": "single_channel",
    "created": "2025-01-01T00:00:00Z"
  },
  "characters": {
    "A": {
      "x": 0,
      "y": 0,
      "width": 64,
      "height": 64,
      "advance": 58.0,
      "bearing_x": 2.0,
      "bearing_y": 62.0
    }
  },
  "shapes": {
    "circle": {
      "x": 0,
      "y": 512,
      "width": 64,
      "height": 64
    }
  }
}
```

## Shader Usage

The generated SDF textures can be used in shaders for high-quality text and shape rendering:

```glsl
// Vertex shader - pass through texture coordinates
attribute vec2 a_position;
attribute vec2 a_texcoord;
varying vec2 v_texcoord;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
}

// Fragment shader - SDF rendering
precision mediump float;
uniform sampler2D u_texture;
uniform float u_buffer;
uniform float u_gamma;
varying vec2 v_texcoord;

void main() {
    float dist = texture2D(u_texture, v_texcoord).a;
    float alpha = smoothstep(u_buffer - u_gamma, u_buffer + u_gamma, dist);
    gl_FragColor = vec4(1.0, 1.0, 1.0, alpha);
}
```

## Library Usage

You can also use this as a library in your Rust projects:

```rust
use sdf_generator::{SdfConfig, SdfGenerator};

let config = SdfConfig {
    font_path: Some("path/to/font.ttf".into()),
    sdf_size: 64,
    atlas_size: 1024,
    sdf_range: 4.0,
    padding: 2,
    include_shapes: true,
    multi_channel: false,
    characters: "ABC123".to_string(),
};

let generator = SdfGenerator::new(config)?;
let result = generator.generate()?;

// Save results
result.sprite_sheet.save("atlas.png")?;
let registry_json = serde_json::to_string_pretty(&result.registry)?;
std::fs::write("registry.json", registry_json)?;
```

## Technical Details

### SDF Generation
- Uses distance field algorithms to create smooth, scalable textures
- Supports both traditional SDF and multi-channel SDF (MSDF) for better corner preservation
- Configurable distance range for balancing quality vs. effect range

### Atlas Packing
- Efficient sprite sheet generation with customizable padding
- Grid-based packing algorithm (can be extended with more sophisticated algorithms)
- Power-of-2 atlas dimensions for optimal GPU usage

### Font Handling
- Pure Rust font parsing using `ttf-parser` (no C dependencies)
- Extracts glyph metrics for proper text layout
- Supports TrueType and OpenType fonts

## Dependencies

- `ttf-parser` - Safe, zero-allocation font parsing
- `image` - Image processing and I/O
- `serde` & `serde_json` - Serialization for registry files
- `clap` - Command-line argument parsing
- `texture_packer` - Texture atlas generation
- `easy-signed-distance-field` - SDF generation algorithms

## License

This project is licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
'''

cargo_toml_updated = '''[package]
name = "sdf_generator"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "A Rust SDF generator for alphanumeric characters and basic shapes"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/sdf-generator"
keywords = ["sdf", "signed-distance-field", "font", "texture-atlas", "shader"]
categories = ["graphics", "game-development", "multimedia::images"]

[dependencies]
ttf-parser = "0.25"
image = "0.25"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
rayon = "1.8"
chrono = { version = "0.4", features = ["serde"] }

# SDF generation
easy-signed-distance-field = "0.1"

# Atlas packing  
texture_packer = "0.30"

[dev-dependencies]
tempfile = "3.0"
criterion = "0.5"

[[bin]]
name = "sdf_generator"
path = "src/main.rs"

[lib]
name = "sdf_generator"
path = "src/lib.rs"

[[bench]]
name = "sdf_generation"
harness = false

[features]
default = []
# Multi-threading support
parallel = ["rayon"]
# Additional SDF algorithms
experimental = []
'''

# Final files dictionary with all content
final_files = {
    "src/utils/mod.rs": utils_mod_rs,
    "src/utils/image.rs": utils_image_rs,
    "src/utils/math.rs": utils_math_rs,
    "README.md": readme_md,
    "Cargo.toml": cargo_toml_updated,
}

print("✅ Utility Modules and Documentation Generated")
print(f"📄 Generated {len(final_files)} final files")
print("📋 Final Files:")
for filename in final_files.keys():
    print(f"   - {filename}")

# Create a comprehensive summary
all_files_count = len(files_content) + len(additional_files) + len(final_files)
print(f"\n🎉 Complete Rust SDF Generator Project Generated!")
print(f"📊 Total Files: {all_files_count}")
print(f"🔧 Core Dependencies: ttf-parser, image, serde, clap")
print(f"⚡ SDF Libraries: easy-signed-distance-field, texture_packer")
print(f"📋 Features: Font SDF generation, Basic shapes, Atlas packing, JSON registry")

# Save summary to CSV for reference
import csv

summary_data = [
    ["Component", "Files", "Description"],
    ["Core", "3", "Main application, library interface, CLI"],
    ["SDF Module", "4", "SDF generation logic, font parsing, shapes"],
    ["Atlas Module", "3", "Texture packing, registry generation"],
    ["Utils Module", "3", "Image processing, math utilities"],
    ["Documentation", "2", "README, updated Cargo.toml"],
    ["Total", str(all_files_count), "Complete SDF generator implementation"]
]

with open('sdf_generator_summary.csv', 'w', newline='') as csvfile:
    writer = csv.writer(csvfile)
    writer.writerows(summary_data)

print(f"📈 Project summary saved to: sdf_generator_summary.csv")