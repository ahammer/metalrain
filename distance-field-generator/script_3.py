# Generate the SDF module implementation files

# SDF module files
sdf_mod_rs = '''pub mod generator;
pub mod shapes;
pub mod font;

pub use generator::*;
pub use shapes::*;
pub use font::*;
'''

sdf_generator_rs = '''use anyhow::Result;
use image::{DynamicImage, GrayImage, RgbImage};

/// Core SDF generation utilities
pub struct SdfGenerator;

impl SdfGenerator {
    /// Generate SDF from a boolean mask image
    pub fn generate_sdf_from_mask(
        mask: &GrayImage,
        range: f32,
    ) -> Result<GrayImage> {
        let (width, height) = mask.dimensions();
        let mut sdf = GrayImage::new(width, height);
        
        // Use easy-signed-distance-field for the actual SDF generation
        // This is a simplified implementation - in practice you'd use the crate
        for y in 0..height {
            for x in 0..width {
                let distance = calculate_distance_to_edge(mask, x, y, range);
                let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
                sdf.put_pixel(x, y, image::Luma([normalized]));
            }
        }
        
        Ok(sdf)
    }
    
    /// Generate multi-channel SDF (MSDF) from a mask
    pub fn generate_msdf_from_mask(
        mask: &GrayImage,
        range: f32,
    ) -> Result<RgbImage> {
        // This would use the fdsm crate or similar for actual MSDF generation
        // For now, we'll simulate by converting single channel to RGB
        let sdf = Self::generate_sdf_from_mask(mask, range)?;
        let (width, height) = sdf.dimensions();
        let mut msdf = RgbImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let gray = sdf.get_pixel(x, y).0[0];
                msdf.put_pixel(x, y, image::Rgb([gray, gray, gray]));
            }
        }
        
        Ok(msdf)
    }
}

/// Calculate distance to nearest edge using a simplified algorithm
fn calculate_distance_to_edge(mask: &GrayImage, x: u32, y: u32, max_range: f32) -> f32 {
    let (width, height) = mask.dimensions();
    let current_pixel = mask.get_pixel(x, y).0[0];
    let is_inside = current_pixel > 128;
    
    let mut min_distance = max_range;
    let search_radius = max_range.ceil() as i32;
    
    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            
            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nx = nx as u32;
                let ny = ny as u32;
                let neighbor_pixel = mask.get_pixel(nx, ny).0[0];
                let neighbor_inside = neighbor_pixel > 128;
                
                if is_inside != neighbor_inside {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    min_distance = min_distance.min(distance);
                }
            }
        }
    }
    
    if is_inside {
        -min_distance
    } else {
        min_distance
    }
}
'''

sdf_shapes_rs = '''use anyhow::Result;
use image::{DynamicImage, GrayImage};
use std::collections::HashMap;

/// Generate SDF images for basic geometric shapes
pub fn generate_basic_shapes(
    size: u32,
    range: f32,
) -> Result<HashMap<String, DynamicImage>> {
    let mut shapes = HashMap::new();
    
    // Generate circle SDF
    let circle = generate_circle_sdf(size, range)?;
    shapes.insert("circle".to_string(), DynamicImage::ImageLuma8(circle));
    
    // Generate triangle SDF  
    let triangle = generate_triangle_sdf(size, range)?;
    shapes.insert("triangle".to_string(), DynamicImage::ImageLuma8(triangle));
    
    // Generate square SDF
    let square = generate_square_sdf(size, range)?;
    shapes.insert("square".to_string(), DynamicImage::ImageLuma8(square));
    
    Ok(shapes)
}

/// Generate SDF for a circle
pub fn generate_circle_sdf(size: u32, range: f32) -> Result<GrayImage> {
    let mut image = GrayImage::new(size, size);
    let center = size as f32 / 2.0;
    let radius = center * 0.8; // 80% of half-size
    
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 - center;
            let py = y as f32 - center;
            
            // Circle SDF: length(p) - radius
            let distance = (px * px + py * py).sqrt() - radius;
            let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
            
            image.put_pixel(x, y, image::Luma([normalized]));
        }
    }
    
    Ok(image)
}

/// Generate SDF for a triangle
pub fn generate_triangle_sdf(size: u32, range: f32) -> Result<GrayImage> {
    let mut image = GrayImage::new(size, size);
    let center = size as f32 / 2.0;
    let scale = center * 0.8;
    
    // Equilateral triangle vertices
    let v0 = (0.0, -0.866); // Top vertex
    let v1 = (-0.75, 0.433); // Bottom left
    let v2 = (0.75, 0.433);  // Bottom right
    
    for y in 0..size {
        for x in 0..size {
            let px = (x as f32 - center) / scale;
            let py = (y as f32 - center) / scale;
            
            // Triangle SDF calculation (simplified)
            let distance = triangle_sdf(px, py, v0, v1, v2) * scale;
            let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
            
            image.put_pixel(x, y, image::Luma([normalized]));
        }
    }
    
    Ok(image)
}

/// Generate SDF for a square/rectangle
pub fn generate_square_sdf(size: u32, range: f32) -> Result<GrayImage> {
    let mut image = GrayImage::new(size, size);
    let center = size as f32 / 2.0;
    let half_size = center * 0.8;
    
    for y in 0..size {
        for x in 0..size {
            let px = (x as f32 - center).abs();
            let py = (y as f32 - center).abs();
            
            // Box SDF: max(abs(p.x) - size.x, abs(p.y) - size.y)
            let distance = (px - half_size).max(py - half_size);
            let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
            
            image.put_pixel(x, y, image::Luma([normalized]));
        }
    }
    
    Ok(image)
}

/// Calculate triangle SDF (simplified implementation)
fn triangle_sdf(px: f32, py: f32, v0: (f32, f32), v1: (f32, f32), v2: (f32, f32)) -> f32 {
    // This is a simplified triangle SDF calculation
    // A complete implementation would use proper edge distance calculations
    
    // Calculate distances to edges
    let d1 = point_to_line_distance(px, py, v0, v1);
    let d2 = point_to_line_distance(px, py, v1, v2);
    let d3 = point_to_line_distance(px, py, v2, v0);
    
    // Check if point is inside triangle (simplified)
    let inside = is_point_in_triangle(px, py, v0, v1, v2);
    let min_dist = d1.min(d2).min(d3);
    
    if inside {
        -min_dist
    } else {
        min_dist
    }
}

fn point_to_line_distance(px: f32, py: f32, p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let length_sq = dx * dx + dy * dy;
    
    if length_sq == 0.0 {
        return ((px - p1.0) * (px - p1.0) + (py - p1.1) * (py - p1.1)).sqrt();
    }
    
    let t = ((px - p1.0) * dx + (py - p1.1) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);
    
    let closest_x = p1.0 + t * dx;
    let closest_y = p1.1 + t * dy;
    
    ((px - closest_x) * (px - closest_x) + (py - closest_y) * (py - closest_y)).sqrt()
}

fn is_point_in_triangle(px: f32, py: f32, v0: (f32, f32), v1: (f32, f32), v2: (f32, f32)) -> bool {
    // Barycentric coordinate method
    let denom = (v1.1 - v2.1) * (v0.0 - v2.0) + (v2.0 - v1.0) * (v0.1 - v2.1);
    let a = ((v1.1 - v2.1) * (px - v2.0) + (v2.0 - v1.0) * (py - v2.1)) / denom;
    let b = ((v2.1 - v0.1) * (px - v2.0) + (v0.0 - v2.0) * (py - v2.1)) / denom;
    let c = 1.0 - a - b;
    
    a >= 0.0 && b >= 0.0 && c >= 0.0
}
'''

sdf_font_rs = '''use anyhow::{anyhow, Result};
use image::{DynamicImage, GrayImage};
use std::collections::HashMap;
use ttf_parser::Face;

#[derive(Debug)]
pub struct CharacterSdf {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// Generate SDF images for all characters in the given string
pub fn generate_character_sdfs(
    font_data: &[u8],
    characters: &str,
    sdf_size: u32,
    range: f32,
    multi_channel: bool,
) -> Result<HashMap<char, CharacterSdf>> {
    let face = Face::parse(font_data, 0)
        .map_err(|e| anyhow!("Failed to parse font: {}", e))?;
    
    let mut results = HashMap::new();
    let scale = sdf_size as f32 / face.units_per_em() as f32;
    
    for ch in characters.chars() {
        if let Some(glyph_id) = face.glyph_index(ch) {
            // Get glyph metrics
            let advance = face.glyph_hor_advance(glyph_id)
                .map(|a| a as f32 * scale)
                .unwrap_or(sdf_size as f32);
            
            let bbox = face.glyph_bounding_box(glyph_id);
            let (bearing_x, bearing_y) = if let Some(bbox) = bbox {
                (
                    bbox.x_min as f32 * scale,
                    bbox.y_max as f32 * scale,
                )
            } else {
                (0.0, 0.0)
            };
            
            // For this example, we'll create a simple placeholder SDF
            // In a real implementation, you'd rasterize the glyph and generate the SDF
            let sdf_image = generate_character_placeholder_sdf(ch, sdf_size, range, multi_channel)?;
            
            let character_sdf = CharacterSdf {
                image: sdf_image,
                width: sdf_size,
                height: sdf_size,
                advance,
                bearing_x,
                bearing_y,
            };
            
            results.insert(ch, character_sdf);
        }
    }
    
    Ok(results)
}

/// Generate a placeholder SDF for a character (for demonstration)
/// In a real implementation, this would rasterize the glyph first
fn generate_character_placeholder_sdf(
    ch: char,
    size: u32,
    range: f32,
    multi_channel: bool,
) -> Result<DynamicImage> {
    // This is a placeholder that creates a simple pattern based on the character
    // In a real implementation, you would:
    // 1. Rasterize the glyph using the font
    // 2. Generate an SDF from the rasterized bitmap
    
    if multi_channel {
        let mut image = image::RgbImage::new(size, size);
        let center = size as f32 / 2.0;
        
        // Create a simple pattern based on character code
        let char_code = ch as u32;
        let pattern_freq = (char_code % 10) as f32 / 10.0 + 0.1;
        
        for y in 0..size {
            for x in 0..size {
                let px = (x as f32 - center) / center;
                let py = (y as f32 - center) / center;
                
                let distance = (px * px + py * py).sqrt() - 0.6;
                let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
                
                // Vary channels slightly for MSDF effect
                let r = normalized;
                let g = (normalized as f32 * (1.0 + pattern_freq * 0.1)) as u8;
                let b = (normalized as f32 * (1.0 - pattern_freq * 0.1)) as u8;
                
                image.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        
        Ok(DynamicImage::ImageRgb8(image))
    } else {
        let mut image = GrayImage::new(size, size);
        let center = size as f32 / 2.0;
        
        for y in 0..size {
            for x in 0..size {
                let px = (x as f32 - center) / center;
                let py = (y as f32 - center) / center;
                
                // Simple circular pattern for demonstration
                let distance = (px * px + py * py).sqrt() - 0.6;
                let normalized = ((distance / range + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
                
                image.put_pixel(x, y, image::Luma([normalized]));
            }
        }
        
        Ok(DynamicImage::ImageLuma8(image))
    }
}
'''

# Atlas module files
atlas_mod_rs = '''pub mod packer;
pub mod registry;

pub use packer::*;
pub use registry::*;
'''

atlas_packer_rs = '''use anyhow::Result;
use image::DynamicImage;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AtlasPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Pack multiple images into a single texture atlas
pub fn pack_atlas(
    images: Vec<(String, DynamicImage)>,
    atlas_size: u32,
    padding: u32,
) -> Result<(DynamicImage, HashMap<String, AtlasPosition>)> {
    // Simple grid-based packing algorithm
    // In a real implementation, you'd use a more sophisticated algorithm
    
    let mut positions = HashMap::new();
    let mut atlas = image::RgbaImage::new(atlas_size, atlas_size);
    
    // Calculate grid dimensions
    let image_count = images.len();
    let grid_size = (image_count as f32).sqrt().ceil() as u32;
    let cell_size = atlas_size / grid_size;
    let content_size = cell_size - padding * 2;
    
    for (i, (id, image)) in images.into_iter().enumerate() {
        let grid_x = (i as u32) % grid_size;
        let grid_y = (i as u32) / grid_size;
        
        let x = grid_x * cell_size + padding;
        let y = grid_y * cell_size + padding;
        
        // Resize image to fit cell
        let resized = image.resize_exact(
            content_size, 
            content_size, 
            image::imageops::FilterType::Lanczos3
        );
        
        // Copy to atlas
        let rgba_image = resized.to_rgba8();
        for py in 0..content_size {
            for px in 0..content_size {
                if x + px < atlas_size && y + py < atlas_size {
                    let source_pixel = rgba_image.get_pixel(px, py);
                    atlas.put_pixel(x + px, y + py, *source_pixel);
                }
            }
        }
        
        positions.insert(id, AtlasPosition {
            x,
            y,
            width: content_size,
            height: content_size,
        });
    }
    
    Ok((DynamicImage::ImageRgba8(atlas), positions))
}
'''

atlas_registry_rs = '''use crate::{SdfRegistry, RegistryMetadata, CharacterInfo, ShapeInfo};
use anyhow::Result;
use std::collections::HashMap;

/// Create registry from atlas packing results
pub fn create_registry(
    atlas_width: u32,
    atlas_height: u32,
    sdf_range: f32,
    resolution: u32,
    padding: u32,
    multi_channel: bool,
    character_positions: HashMap<char, (u32, u32, u32, u32)>,
    shape_positions: HashMap<String, (u32, u32, u32, u32)>,
) -> Result<SdfRegistry> {
    let mut characters = HashMap::new();
    let mut shapes = HashMap::new();
    
    // Convert character positions
    for (ch, (x, y, width, height)) in character_positions {
        characters.insert(ch, CharacterInfo {
            x,
            y,
            width,
            height,
            advance: width as f32, // Simplified
            bearing_x: 0.0,
            bearing_y: height as f32,
        });
    }
    
    // Convert shape positions
    for (name, (x, y, width, height)) in shape_positions {
        shapes.insert(name, ShapeInfo {
            x,
            y,
            width,
            height,
        });
    }
    
    let registry = SdfRegistry {
        metadata: RegistryMetadata {
            atlas_width,
            atlas_height,
            sdf_range,
            resolution,
            padding,
            format: if multi_channel { 
                "multi_channel".to_string() 
            } else { 
                "single_channel".to_string() 
            },
            created: chrono::Utc::now().to_rfc3339(),
        },
        characters,
        shapes,
    };
    
    Ok(registry)
}
'''

# Create additional files content dictionary
additional_files = {
    "src/sdf/mod.rs": sdf_mod_rs,
    "src/sdf/generator.rs": sdf_generator_rs,
    "src/sdf/shapes.rs": sdf_shapes_rs,
    "src/sdf/font.rs": sdf_font_rs,
    "src/atlas/mod.rs": atlas_mod_rs,
    "src/atlas/packer.rs": atlas_packer_rs,
    "src/atlas/registry.rs": atlas_registry_rs,
}

print("✅ SDF and Atlas Module Files Generated")
print(f"📄 Generated {len(additional_files)} additional files")
print("📋 Module Files:")
for filename in additional_files.keys():
    print(f"   - {filename}")