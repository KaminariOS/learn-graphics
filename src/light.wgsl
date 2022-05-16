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
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let scale = 0.25;
    var v_out: VertexOutput;
    v_out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + light.position.xyz, 1.0);
    v_out.color = light.color.rgb;
    return v_out;
}

[[stage(fragment)]]
fn fs_main(f_in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(f_in.color, 1.0);
}