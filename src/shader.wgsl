// Vertex shader

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var v_out: VertexOutput;
    v_out.color = model.color;
    v_out.clip_position = vec4<f32>(model.position, 1.0);
    return v_out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(v_in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(v_in.color, 1.0);
}