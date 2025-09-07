// Vertex Shader
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
