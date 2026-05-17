struct Uniforms {
    mvp: mat4x4<f32>,
    color: vec4<f32>,
    is_occluder: f32, // 1.0 = casts shadow, 0.0 = no shadow
}

@group(0) @binding(0) var<uniform> u_uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_uniforms.mvp * vec4<f32>(position, 0.0, 1.0);
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) occlusion: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = u_uniforms.color;
    
    // Shapes are geometrically solid, so opacity comes from color alpha.
    // If alpha < 0.5 we consider it non-blocking? Or just block anyway?
    // Let's stick to is_occluder flag.
    out.occlusion = vec4<f32>(u_uniforms.is_occluder, 0.0, 0.0, 1.0);
    
    return out;
}

