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
    cutoff_inner_outer_eps: vec4<f32>,
    view_proj: mat4x4<f32>,
}



@group(0) @binding(0)
var<uniform> light: Light;


struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
};

@vertex
fn vs_bake(model: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3
    );
    return light.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
}