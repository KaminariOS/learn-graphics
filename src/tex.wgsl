struct VertexInput_tex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput_tex {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main_tex(
    model: VertexInput_tex
) -> VertexOutput_tex {
    var v_out: VertexOutput;
    v_out.tex_coords = model.tex_coords;
    v_out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return v_out;
}

[[stage(fragment)]]
fn fs_main_tex(f_in: VertexOutput) -> [[location(0)]] vec4<f32> {
//    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vec4<f32>(f_in.tex_coords, 1.0, 1.0);
}
