# Rust SDF Generator - Build and Usage Guide

## Project Overview

This Rust project generates signed distance fields (SDFs) for alphanumeric characters and basic geometric shapes, creating optimized sprite sheets and registry files for shader usage.

## Quick Start

### 1. Project Setup

Create a new Rust project and add the dependencies:

```bash
cargo new sdf_generator
cd sdf_generator
```

Copy all the provided source files into their respective directories:

```
sdf_generator/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── sdf/
│   │   ├── mod.rs
│   │   ├── generator.rs
│   │   ├── font.rs
│   │   └── shapes.rs
│   ├── atlas/
│   │   ├── mod.rs
│   │   ├── packer.rs
│   │   └── registry.rs
│   └── utils/
│       ├── mod.rs
│       ├── image.rs
│       └── math.rs
├── examples/
├── tests/
└── fonts/
    └── (place your TTF/OTF files here)
```

### 2. Build the Project

```bash
cargo build --release
```

### 3. Run Examples

Basic character SDF generation:
```bash
cargo run -- --font-path fonts/Arial.ttf --output-dir output
```

Include shapes with multi-channel SDF:
```bash
cargo run -- --font-path fonts/Arial.ttf --shapes --multi-channel --output-dir output
```

## Detailed Usage

### Command Line Interface

The CLI provides extensive customization options:

```bash
sdf_generator [OPTIONS]

OPTIONS:
    -f, --font-path <PATH>        Font file to generate SDFs from
    -o, --output-dir <DIR>        Output directory [default: ./output]
        --sdf-size <SIZE>         SDF texture size per glyph [default: 64]
        --atlas-size <SIZE>       Atlas dimensions [default: 1024]
        --sdf-range <RANGE>       Distance field range in pixels [default: 4]
        --padding <PIXELS>        Padding between glyphs [default: 2]
        --shapes                  Include basic shapes
        --multi-channel           Use MSDF instead of single channel
        --characters <CHARS>      Custom character set [default: a-zA-Z0-9]
    -h, --help                    Print help information
    -V, --version                 Print version information
```

### Library Usage

You can integrate the SDF generator as a library:

```rust
use sdf_generator::{SdfConfig, SdfGenerator};

fn generate_sdf_atlas() -> anyhow::Result<()> {
    let config = SdfConfig {
        font_path: Some("fonts/MyFont.ttf".into()),
        sdf_size: 128,
        atlas_size: 2048,
        sdf_range: 8.0,
        padding: 4,
        include_shapes: true,
        multi_channel: false,
        characters: "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string(),
    };

    let generator = SdfGenerator::new(config)?;
    let result = generator.generate()?;

    // Save outputs
    result.sprite_sheet.save("my_sdf_atlas.png")?;
    
    let registry_json = serde_json::to_string_pretty(&result.registry)?;
    std::fs::write("my_sdf_registry.json", registry_json)?;

    Ok(())
}
```

## Implementation Details

### SDF Generation Algorithm

The project supports two main SDF types:

1. **Single-Channel SDF**: Traditional grayscale distance fields
   - Pros: Smaller memory footprint, simple implementation
   - Cons: Rounded corners, limited quality at small sizes
   - Use: Basic text rendering without sharp corner requirements

2. **Multi-Channel SDF (MSDF)**: RGB channels for better corner preservation
   - Pros: Sharp corners, higher quality, better scaling
   - Cons: More complex, larger textures
   - Use: High-quality text rendering with sharp corners

### Atlas Packing

The texture atlas packer uses a grid-based algorithm that:
- Arranges SDFs in a regular grid with configurable padding
- Supports power-of-2 atlas dimensions for GPU optimization
- Calculates precise UV coordinates for each glyph/shape
- Can be extended with more sophisticated packing algorithms

### Registry Format

The JSON registry provides all necessary data for shader usage:

```json
{
  "metadata": {
    "atlas_width": 1024,
    "atlas_height": 1024,
    "sdf_range": 4.0,
    "resolution": 64,
    "padding": 2,
    "format": "single_channel"
  },
  "characters": {
    "A": {
      "x": 0, "y": 0,
      "width": 64, "height": 64,
      "advance": 58.0,
      "bearing_x": 2.0, "bearing_y": 62.0
    }
  },
  "shapes": {
    "circle": {
      "x": 0, "y": 512,
      "width": 64, "height": 64
    }
  }
}
```

## Shader Integration

### GLSL Fragment Shader

```glsl
precision mediump float;

uniform sampler2D u_sdf_atlas;
uniform float u_buffer;
uniform float u_gamma;
uniform vec3 u_color;

varying vec2 v_texcoord;

void main() {
    float distance = texture2D(u_sdf_atlas, v_texcoord).a;
    float alpha = smoothstep(u_buffer - u_gamma, u_buffer + u_gamma, distance);
    gl_FragColor = vec4(u_color, alpha);
}
```

### JavaScript Integration

```javascript
// Load registry and atlas
const registry = await fetch('sdf_registry.json').then(r => r.json());
const atlasTexture = loadTexture('sdf_atlas.png');

// Render character
function renderCharacter(char, x, y, scale) {
    const info = registry.characters[char];
    if (!info) return;
    
    const u1 = info.x / registry.metadata.atlas_width;
    const v1 = info.y / registry.metadata.atlas_height;
    const u2 = (info.x + info.width) / registry.metadata.atlas_width;
    const v2 = (info.y + info.height) / registry.metadata.atlas_height;
    
    // Set up quad with these UV coordinates
    renderQuad(x, y, info.width * scale, info.height * scale, u1, v1, u2, v2);
}
```

## Performance Considerations

### Memory Usage
- Single-channel SDF: ~1/3 the memory of MSDF
- Atlas size directly impacts GPU memory usage
- Padding reduces texture bleeding but increases atlas size

### Generation Speed
- Font parsing: ~1ms per character
- SDF generation: ~1-10ms per character (depends on resolution)
- Atlas packing: ~1ms per atlas
- Parallel processing available via `rayon` feature

### Quality vs Size Trade-offs
- Higher SDF resolution = better quality but larger atlases
- Larger distance range = more effects possible but lower precision
- More padding = less bleeding but reduced packing efficiency

## Advanced Configuration

### Custom Packing Algorithms

Extend the atlas packer with more sophisticated algorithms:

```rust
pub trait AtlasPacker {
    fn pack(&self, images: Vec<(String, DynamicImage)>) -> Result<(DynamicImage, HashMap<String, AtlasPosition>)>;
}

// Implement custom packing strategies
struct MaxRectsAtlasPacker;
struct GuillotineAtlasPacker;
struct ShelfAtlasPacker;
```

### Custom SDF Algorithms

Add support for different SDF generation methods:

```rust
pub trait SdfGenerator {
    fn generate_from_mask(&self, mask: &GrayImage, range: f32) -> Result<DynamicImage>;
}

// Implement various SDF algorithms
struct JumpFloodSdfGenerator;
struct EuclideanSdfGenerator;
struct MsdfGenerator;
```

## Troubleshooting

### Common Issues

1. **Font not found**: Ensure font path is correct and font is readable
2. **Atlas too small**: Increase atlas size or reduce SDF size
3. **Blurry output**: Increase SDF resolution or adjust distance range
4. **Performance issues**: Enable parallel processing or reduce character set

### Debug Output

Enable debug logging:

```bash
RUST_LOG=debug cargo run -- --font-path font.ttf
```

### Testing

Run unit tests:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

## Future Enhancements

### Planned Features
- [ ] Better SDF algorithms (Jump Flood Algorithm)
- [ ] More sophisticated atlas packing
- [ ] Variable font support
- [ ] Kerning information in registry
- [ ] Glyph subsetting optimization
- [ ] WebAssembly target support
- [ ] GPU-accelerated SDF generation

### Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is dual-licensed under MIT OR Apache-2.0.