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
    ambient_strength: f32;
    specular_strength: f32;
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
            instance.normal_matrix_2
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

fn multisample_tex(tex_coords: vec2<f32>, sample_count: f32) -> vec4<f32> {
    let tex_c = vec2<f32>(tex_coords.x % 1.0, 1.0 - tex_coords.y % 1.0);
    let normal_tex_cord = vec2<i32>(vec2<f32>(textureDimensions(t_diffuse)) * tex_c);
    var obj_color = vec4<f32>(0.0);
    let sample_count: f32 = 1.0;
    for (var i: i32 = 0; i < i32(sample_count); i = i + 1) {
      obj_color = obj_color + textureLoad(t_diffuse, normal_tex_cord, i);
    }
    obj_color = obj_color / sample_count;
    return obj_color;
}
[[stage(fragment)]]
fn fs_main(f_in: VertexOutput) -> [[location(0)]] vec4<f32> {

     let dis = length(light.position.xyz - f_in.world_position);
     let light_color = light.color.rgb / (1.0 + 0.045 * dis + 0.0075 * dis * dis);
     let ambient_strength = light.ambient_strength;
     let ambient_color = light_color * ambient_strength;
     let light_dir = normalize(light.position.xyz - f_in.world_position);
     let view_dir = normalize(camera.view_pos.xyz - f_in.world_position);
     let half_dir = normalize(view_dir + light_dir);

     let diffuse_strength = max(dot(f_in.world_normal, light_dir), 0.0);
     let diffuse_color = light_color * diffuse_strength;

     let specular_strength = pow(max(dot(f_in.world_normal, half_dir), 0.0), 32.0);
     let specular_color = light.specular_strength * specular_strength * light_color;

    let v_tex = vec2<f32>(f_in.tex_coords.x, 1.0 - f_in.tex_coords.y);
    let obj_color = textureSample(t_diffuse, s_diffuse, v_tex);
    let res = (ambient_color + diffuse_color + specular_color) * obj_color.xyz;
    return vec4<f32>(res, obj_color.a);
}