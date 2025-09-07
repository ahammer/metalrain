# Create a sample registry JSON file that demonstrates the structure
import json

sample_registry = {
    "metadata": {
        "atlas_width": 1024,
        "atlas_height": 1024,
        "sdf_range": 4.0,
        "resolution": 64,
        "padding": 2,
        "format": "single_channel",
        "created": "2025-09-06T20:43:00Z"
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
        },
        "B": {
            "x": 66,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 54.0,
            "bearing_x": 4.0,
            "bearing_y": 62.0
        },
        "C": {
            "x": 132,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 56.0,
            "bearing_x": 3.0,
            "bearing_y": 62.0
        },
        "a": {
            "x": 198,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 52.0,
            "bearing_x": 1.0,
            "bearing_y": 42.0
        },
        "b": {
            "x": 264,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 54.0,
            "bearing_x": 2.0,
            "bearing_y": 62.0
        },
        "1": {
            "x": 330,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 38.0,
            "bearing_x": 8.0,
            "bearing_y": 62.0
        },
        "2": {
            "x": 396,
            "y": 0,
            "width": 64,
            "height": 64,
            "advance": 52.0,
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

# Save the sample registry
with open('sample_sdf_registry.json', 'w') as f:
    json.dump(sample_registry, f, indent=2)

print("✅ Sample Registry JSON Created")
print(f"📋 Characters: {len(sample_registry['characters'])}")
print(f"🔶 Shapes: {len(sample_registry['shapes'])}")
print(f"📏 Atlas Size: {sample_registry['metadata']['atlas_width']}x{sample_registry['metadata']['atlas_height']}")
print(f"📐 SDF Resolution: {sample_registry['metadata']['resolution']}px")
print(f"📊 Distance Range: {sample_registry['metadata']['sdf_range']} pixels")

# Create shader usage example
shader_example = '''// Vertex Shader
attribute vec2 a_position;
attribute vec2 a_texcoord;
varying vec2 v_texcoord;

uniform mat4 u_projection;
uniform mat4 u_view;
uniform mat4 u_model;

void main() {
    gl_Position = u_projection * u_view * u_model * vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
}

// Fragment Shader for SDF rendering
precision mediump float;

uniform sampler2D u_sdf_atlas;
uniform float u_buffer;       // Typically 0.5 for sharp edges
uniform float u_gamma;        // Anti-aliasing width
uniform vec3 u_color;
uniform float u_outline_width;
uniform vec3 u_outline_color;

varying vec2 v_texcoord;

void main() {
    // Sample the SDF
    float distance = texture2D(u_sdf_atlas, v_texcoord).a;
    
    // Calculate opacity for main shape
    float alpha = smoothstep(u_buffer - u_gamma, u_buffer + u_gamma, distance);
    
    // Calculate outline
    float outline_alpha = smoothstep(
        u_buffer - u_outline_width - u_gamma, 
        u_buffer - u_outline_width + u_gamma, 
        distance
    );
    
    // Combine main color and outline
    vec3 final_color = mix(u_outline_color, u_color, alpha);
    float final_alpha = max(alpha, outline_alpha);
    
    gl_FragColor = vec4(final_color, final_alpha);
}

// JavaScript usage example
const registry = await fetch('sdf_registry.json').then(r => r.json());
const atlas = new Image();
atlas.src = 'sdf_atlas.png';

// Render character 'A'
const charA = registry.characters['A'];
const texCoords = [
    charA.x / registry.metadata.atlas_width,                    // left
    charA.y / registry.metadata.atlas_height,                   // top
    (charA.x + charA.width) / registry.metadata.atlas_width,    // right
    (charA.y + charA.height) / registry.metadata.atlas_height   // bottom
];

// Set up geometry with these texture coordinates
setupCharacterGeometry(texCoords, charA.advance, charA.bearing_x, charA.bearing_y);
'''

with open('shader_usage_example.glsl', 'w') as f:
    f.write(shader_example)

print("📜 Shader usage example created: shader_usage_example.glsl")