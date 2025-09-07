# Create a detailed technical specification for the SDF generator project
project_spec = {
    "project_name": "Rust SDF Generator",
    "description": "A Rust application that generates signed distance fields for alphanumeric characters and basic shapes, outputting sprite sheets and registry files for shader usage.",
    
    "dependencies": {
        "core": [
            "ttf-parser = \"0.25\"  # Safe font parsing",
            "image = \"0.25\"  # Image I/O and manipulation", 
            "serde = { version = \"1.0\", features = [\"derive\"] }  # Serialization",
            "serde_json = \"1.0\"  # JSON support",
            "clap = { version = \"4.0\", features = [\"derive\"] }  # CLI interface"
        ],
        "sdf_generation": [
            "easy-signed-distance-field = \"0.1\"  # Pure Rust SDF generation",
            "# Alternative: sdf_glyph_renderer = \"0.4\"  # FreeType-based option"
        ],
        "atlas_packing": [
            "texture_packer = \"0.30\"  # Texture atlas generation",
            "# Alternative: etagere = \"0.2\"  # Dynamic allocation option"
        ],
        "optional": [
            "rayon = \"1.8\"  # Parallel processing",
            "anyhow = \"1.0\"  # Error handling"
        ]
    },
    
    "output_formats": {
        "sprite_sheet": {
            "format": "PNG",
            "bit_depth": "8-bit grayscale or RGB (for MSDF)",
            "dimensions": "Power of 2 (512x512, 1024x1024, 2048x2048)",
            "padding": "2-4 pixels between glyphs"
        },
        "registry": {
            "format": "JSON",
            "structure": {
                "metadata": {
                    "atlas_width": "number",
                    "atlas_height": "number", 
                    "sdf_range": "number (distance field range in pixels)",
                    "resolution": "number (SDF resolution)",
                    "padding": "number"
                },
                "characters": {
                    "a-z, A-Z, 0-9": {
                        "x": "number (atlas x coordinate)", 
                        "y": "number (atlas y coordinate)",
                        "width": "number (glyph width)",
                        "height": "number (glyph height)",
                        "advance": "number (character advance width)",
                        "bearing_x": "number (horizontal bearing)",
                        "bearing_y": "number (vertical bearing)"
                    }
                },
                "shapes": {
                    "circle/triangle/square": {
                        "x": "number",
                        "y": "number", 
                        "width": "number",
                        "height": "number"
                    }
                }
            }
        }
    },
    
    "sdf_algorithms": {
        "single_channel": {
            "description": "Traditional SDF using single grayscale channel",
            "pros": ["Simple implementation", "Smaller memory footprint"],
            "cons": ["Rounded corners", "Limited quality at small sizes"],
            "use_case": "Basic text rendering without sharp corner requirements"
        },
        "multi_channel": {
            "description": "MSDF using RGB channels for better corner preservation",
            "pros": ["Sharp corners", "Higher quality", "Better scaling"],
            "cons": ["More complex", "Larger textures", "RGB instead of grayscale"],
            "use_case": "High-quality text rendering with sharp corners"
        }
    },
    
    "geometric_shapes": {
        "circle": {
            "sdf_function": "length(p) - radius",
            "parameters": ["center_x", "center_y", "radius"]
        },
        "rectangle": {
            "sdf_function": "max(abs(p.x) - size.x, abs(p.y) - size.y)",
            "parameters": ["center_x", "center_y", "width", "height"]
        },
        "triangle": {
            "sdf_function": "Complex function involving dot products and clamping",
            "parameters": ["vertex1", "vertex2", "vertex3"]
        }
    },
    
    "project_structure": [
        "src/",
        "  main.rs           # CLI interface and main application logic",
        "  lib.rs            # Library interface",
        "  sdf/",
        "    mod.rs           # SDF module exports",
        "    generator.rs     # Core SDF generation logic",
        "    shapes.rs        # Geometric shape SDF functions",
        "    font.rs          # Font glyph SDF generation", 
        "  atlas/",
        "    mod.rs           # Atlas module exports",
        "    packer.rs        # Sprite sheet packing logic",
        "    registry.rs      # Registry file generation",
        "  utils/",
        "    mod.rs           # Utility module exports",
        "    image.rs         # Image processing utilities",
        "    math.rs          # Mathematical helper functions",
        "Cargo.toml          # Project configuration",
        "README.md           # Documentation",
        "examples/           # Usage examples",
        "tests/              # Unit tests"
    ],
    
    "cli_interface": {
        "basic_usage": "sdf_generator --font-path font.ttf --output-dir ./output",
        "options": [
            "--font-path <PATH>      Font file to generate SDFs from",
            "--output-dir <DIR>      Output directory for sprite sheet and registry",
            "--sdf-size <SIZE>       SDF texture size (default: 64)",
            "--atlas-size <SIZE>     Atlas dimensions (default: 1024)", 
            "--sdf-range <RANGE>     Distance field range in pixels (default: 4)",
            "--padding <PIXELS>      Padding between glyphs (default: 2)",
            "--shapes                Include basic shapes (circle, triangle, square)",
            "--multi-channel         Use multi-channel SDF (MSDF) instead of single channel",
            "--characters <CHARS>    Custom character set (default: a-zA-Z0-9)"
        ]
    }
}

# Generate example registry JSON structure
registry_example = {
    "metadata": {
        "atlas_width": 1024,
        "atlas_height": 1024,
        "sdf_range": 4,
        "resolution": 64,
        "padding": 2,
        "format": "single_channel",  # or "multi_channel"
        "created": "2025-01-01T00:00:00Z"
    },
    "characters": {
        "A": {
            "x": 0,
            "y": 0, 
            "width": 64,
            "height": 64,
            "advance": 58,
            "bearing_x": 2,
            "bearing_y": 62
        },
        "B": {
            "x": 66,
            "y": 0,
            "width": 64, 
            "height": 64,
            "advance": 54,
            "bearing_x": 4,
            "bearing_y": 62
        },
        # ... more characters
    },
    "shapes": {
        "circle": {
            "x": 0,
            "y": 512,
            "width": 64,
            "height": 64
        },
        "triangle": {
            "x": 66,
            "y": 512,
            "width": 64,
            "height": 64  
        },
        "square": {
            "x": 132,
            "y": 512,
            "width": 64,
            "height": 64
        }
    }
}

print("✅ Project Specification Generated")
print(f"📦 Dependencies: {len(project_spec['dependencies']['core'])} core + {len(project_spec['dependencies']['sdf_generation'])} SDF")
print(f"📁 Project Structure: {len(project_spec['project_structure'])} files/directories")
print(f"⚙️  CLI Options: {len(project_spec['cli_interface']['options'])} command-line options")
print(f"📊 Registry Example: {len(registry_example['characters'])} character entries + {len(registry_example['shapes'])} shapes")