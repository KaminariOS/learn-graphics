// Vertex shader
struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct Light {
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
var<storage, read> lights: Lights;

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
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

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

struct MaterialUniform {
    ambient: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32
};
@group(3) @binding(0)
var<uniform> material_uniform: MaterialUniform;

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

fn attenuation(light: Light, world_position: vec3<f32>) -> vec3<f32> {
    var light_color = light.color.rgb;
    if light.point_clq[3] != 0.0 {
            let dis = length(light.position - world_position);
            light_color = light_color / (light.point_clq[0] + light.point_clq[1] * dis + light.point_clq[2] * dis * dis);
    }
    return light_color;
}

fn cutoff(light: Light, dir: vec3<f32>) -> f32 {
    if light.cutoff_inner_outer_eps[3] == 0.0 {
        return 1.0;
    }
    let theta = dot(-light.direction, dir);
    let epsilon = light.cutoff_inner_outer_eps[2];
    let intensity = clamp((theta - light.cutoff_inner_outer_eps[1]) / epsilon, 0.0, 1.0);
    return intensity;
}

@fragment
fn fs_main(f_in: VertexOutput) -> @location(0) vec4<f32> {
     var res = vec3<f32>(0.);
     var light_count = i32(arrayLength(&lights.lights));
     let v_tex = vec2<f32>(f_in.tex_coords.x, 1.0 - f_in.tex_coords.y);
     let obj_color = textureSample(t_diffuse, s_diffuse, v_tex);

     for(var i: i32 = 0; i < light_count; i++) {
     let light = lights.lights[i];
     let dis = length(light.position - f_in.world_position);
     let light_color = attenuation(light, f_in.world_position);
     let ambient_strength = light.ambient_strength;
     let ambient_color = light_color * ambient_strength * material_uniform.ambient;
     let light_dir = normalize(light.position - f_in.world_position);
     let view_dir = normalize(camera.view_pos.xyz - f_in.world_position);
     let half_dir = normalize(view_dir + light_dir);

     let cut_off_intensity = cutoff(light, -light_dir);

     let diffuse_strength = max(dot(f_in.world_normal, light_dir), 0.0);
     let diffuse_color = light_color * diffuse_strength * material_uniform.diffuse * cut_off_intensity;

     let specular_strength = pow(max(dot(f_in.world_normal, half_dir), 0.0), 32.0);
     let specular_color = light.specular_strength * specular_strength * light_color * material_uniform.specular * cut_off_intensity;

    let v_tex = vec2<f32>(f_in.tex_coords.x, 1.0 - f_in.tex_coords.y);
    let obj_color = textureSample(t_diffuse, s_diffuse, v_tex);
    res += (ambient_color + diffuse_color + specular_color) * obj_color.xyz;
     }
    return vec4<f32>(res, obj_color.a);
}