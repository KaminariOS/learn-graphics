struct CameraUniform {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]] // 1.
var<uniform> camera: CameraUniform;

struct VertexInput_tex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput_tex {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput_tex
) -> VertexOutput_tex {
    var v_out: VertexOutput_tex;
    v_out.tex_coords = model.tex_coords;
    v_out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return v_out;
}

[[group(1), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(1), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(f_in: VertexOutput_tex) -> [[location(0)]] vec4<f32> {
//    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return textureSample(t_diffuse, s_diffuse, f_in.tex_coords);
}
