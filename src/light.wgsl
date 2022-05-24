struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>
};

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct Light {
    // pos_dir[3] == 1? position: direction
    position: vec3<f32>,
    direction: vec3<f32>,
    color: vec4<f32>,
    diffuse_strength: f32,
    ambient_strength: f32,
    specular_strength: f32,
    // constant, linear, quadratic
    // point_clq[4] == 0? no_attenuation: attenuation
    point_clq: vec4<f32>,
    // cutoff_inner_outer_eps[4] == 0? no_cutoff: cutoff
    cutoff_inner_outer_eps: vec4<f32>
}

struct Lights {
    lights: array<Light>
}

@group(1) @binding(0)
var<storage, read> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    // let light = lights.lights[0];
    let scale = 0.25;
    var v_out: VertexOutput;
    v_out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + light.position, 1.0);
    v_out.color = light.color;
    return v_out;
}

@fragment
fn fs_main(f_in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(f_in.color);
}