struct Uniforms {
    mvp: mat4x4<f32>,
    color: vec4<f32>,
    uv_offset: vec2<f32>,
    uv_scale: vec2<f32>,
    is_occluder: f32, // 1.0 = casts shadow, 0.0 = no shadow
}

@group(0) @binding(0) var<uniform> u_uniforms: Uniforms;
@group(0) @binding(1) var sprite_tex: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_uniforms.mvp * vec4<f32>(position, 0.0, 1.0);
    // Apply UV transform (scale then offset)
    out.uv = uv * u_uniforms.uv_scale + u_uniforms.uv_offset;
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) occlusion: vec4<f32>, // R8 format effectively, but writing vec4
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    let tex_color = textureSample(sprite_tex, sprite_sampler, in.uv);
    let final_color = tex_color * u_uniforms.color;
    
    out.color = final_color;
    
    // Occlusion output:
    // If is_occluder is 1.0 and alpha > threshold, write 1.0 implies occlusion.
    // If is_occluder is 0.0, write 0.0 (no occlusion).
    // Use alpha threshold of 0.5 for occlusion to match previous logic
    let is_opaque = step(0.5, final_color.a);
    let occlusion_val = u_uniforms.is_occluder * is_opaque;
    
    out.occlusion = vec4<f32>(occlusion_val, 0.0, 0.0, 1.0);
    
    return out;
}
