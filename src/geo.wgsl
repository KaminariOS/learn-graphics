// Vertex shader
struct CameraUniform {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]] // 1.
var<uniform> camera: CameraUniform;

struct Light {
    position: vec4<f32>;
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> light: Light;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};
struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;
    [[location(9)]] normal_matrix_0: vec3<f32>;
    [[location(10)]] normal_matrix_1: vec3<f32>;
    [[location(11)]] normal_matrix_2: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] world_position: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3
    );
    let normal_matrix = mat3x3<f32>(
            instance.normal_matrix_0,
            instance.normal_matrix_1,
            instance.normal_matrix_2,
        );
    var v_out: VertexOutput;
    v_out.tex_coords = model.tex_coords;
    v_out.world_normal = normal_matrix * model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    v_out.world_position = world_position.xyz;
    v_out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return v_out;
}

// Fragment shader

[[group(2), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(2), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(f_in: VertexOutput) -> [[location(0)]] vec4<f32> {
     let ambient_strength = 0.1;
     let ambient_color = light.color.xyz * ambient_strength;

     let light_dir = normalize(light.position.xyz - f_in.world_position);
     let view_dir = normalize(camera.view_pos.xyz - f_in.world_position);
     let half_dir = normalize(view_dir + light_dir);

     let diffuse_strength = max(dot(f_in.world_normal, light_dir), 0.0);
     let diffuse_color = light.color.rgb * diffuse_strength;

     let specular_strength = pow(max(dot(f_in.world_normal, half_dir), 0.0), 32.0);
     let specular_color = specular_strength * light.color.rgb;

    let v_tex = vec2<f32>(f_in.tex_coords.x, 1.0 - f_in.tex_coords.y);
    let obj_color = textureSample(t_diffuse, s_diffuse, v_tex);
    let res = (ambient_color + diffuse_color + specular_color) * obj_color.xyz;
    return vec4<f32>(res, obj_color.a);
}