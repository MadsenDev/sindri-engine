// Composite shader: multiplies scene texture with light map texture

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var light_map_tex: texture_2d<f32>;
@group(0) @binding(3) var light_map_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene_color = textureSample(scene_tex, scene_sampler, in.uv);
    let light_map = textureSample(light_map_tex, light_map_sampler, in.uv);

    // Apply lighting: scene * (ambient + light_map)
    // Light map accumulates lights additively (black = no light, white/colored = light)
    // Add ambient so unlit areas aren't completely black
    let ambient = 0.25; // Increased ambient for visibility
    let light_brightness = vec3<f32>(ambient) + light_map.rgb;

    // Multiply scene color with light brightness to apply lighting
    // This makes lit areas brighter and unlit areas darker
    return vec4<f32>(scene_color.rgb * light_brightness, scene_color.a);
}

