// Light rendering shader for 2D point lights and spotlights with shadows

struct LightUniforms {
    position: vec2<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    falloff: f32,
    direction: vec2<f32>, // Spotlight direction (normalized), or [0,0] for point light
    angle: f32, // Spotlight angle (cos of half-angle), or 0 for point light
    screen_size: vec2<f32>,
    view_proj: mat4x4<f32>,
    mvp: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: LightUniforms;
@group(0) @binding(1) var occlusion_tex: texture_2d<f32>; // R8 texture
@group(0) @binding(2) var occlusion_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Input position is in local space (-1 to 1 for the quad)
    // Transform to world space: scale by radius and translate by light position
    let local_pos = in.position;
    let world_pos_2d = uniforms.position + local_pos * uniforms.radius;
    
    // Transform to clip space using MVP (which includes view-projection)
    // MVP = view_proj * translation * scale
    // So we apply MVP to local_pos to get clip space
    let local_pos_vec4 = vec4<f32>(local_pos, 0.0, 1.0);
    out.clip_position = uniforms.mvp * local_pos_vec4;
    
    // World position for fragment shader (used for distance calculations)
    out.world_position = world_pos_2d;
    return out;
}

// Convert world position to screen UV coordinates
fn world_to_screen_uv(world_pos: vec2<f32>) -> vec2<f32> {
    let clip_pos = uniforms.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    // Convert from clip space [-1,1] to UV [0,1]
    let uv = clip_pos.xy / clip_pos.w;
    return vec2<f32>(0.5) + uv * vec2<f32>(0.5);
}

// Check if a point is occluded by sampling the occlusion texture
fn is_occluded(world_pos: vec2<f32>) -> bool {
    let uv = world_to_screen_uv(world_pos);
    let occlusion_sample = textureSample(occlusion_tex, occlusion_sampler, uv);
    // R channel > 0.5 means occluded
    return occlusion_sample.r > 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate distance from light center in world space
    let dist = distance(in.world_position, uniforms.position);

    // Early exit if beyond radius (don't draw anything)
    if dist > uniforms.radius {
        discard;
    }

    // Spotlight support: if a direction is provided, clamp light to the cone
    var light_strength = uniforms.intensity;
    let dir_len = length(uniforms.direction);
    if dir_len > 0.0 {
        let light_dir = normalize(uniforms.direction);
        let to_fragment = normalize(in.world_position - uniforms.position);
        let alignment = dot(to_fragment, light_dir);

        // Discard fragments outside the cone
        if alignment < uniforms.angle {
            discard;
        }

        // Smooth edge for the spotlight cone
        let spot_falloff = clamp((alignment - uniforms.angle) / (1.0 - uniforms.angle), 0.0, 1.0);
        light_strength *= spot_falloff;
    }

    // Calculate distance falloff (1.0 at center, 0.0 at radius)
    let normalized_dist = dist / uniforms.radius;
    let distance_falloff = pow(clamp(1.0 - normalized_dist, 0.0, 1.0), uniforms.falloff);
    light_strength *= distance_falloff;

    // Shadowing/occlusion: check if there's an occluder between light and fragment
    // Cast a ray from light to fragment and sample along it
    let light_to_fragment = in.world_position - uniforms.position;
    let ray_length = length(light_to_fragment);
    let ray_dir = light_to_fragment / ray_length;
    
    // Sample along the ray (skip the endpoint to avoid self-occlusion)
    const SHADOW_SAMPLES: i32 = 8;
    var shadowed = false;
    for (var i: i32 = 1; i < SHADOW_SAMPLES; i++) {
        let t = (f32(i) / f32(SHADOW_SAMPLES)) * ray_length;
        // Skip very close to light to avoid self-shadowing
        if t < 2.0 {
            continue;
        }
        let sample_pos = uniforms.position + ray_dir * t;
        if is_occluded(sample_pos) {
            shadowed = true;
            break;
        }
    }
    
    if shadowed {
        light_strength *= 0.1; // In shadow - very dim
    }

    // Apply light color - mix between white (neutral) and light color
    let light_color = mix(vec3<f32>(1.0), uniforms.color, 0.6);

    // Return light contribution (will be accumulated additively in light map)
    return vec4<f32>(light_color * light_strength, 1.0);
}

