struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
        // from world to camera
    view: mat4x4<f32>,
};

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;


@group(1)
@binding(0)
var r_texture: texture_cube<f32>;
@group(1)
@binding(1)
var r_sampler: sampler;

struct SkyOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec3<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) vertex_index: u32) -> SkyOutput {
    // hacky way to draw a large triangle
    let tmp1 = i32(vertex_index) / 2;
    let tmp2 = i32(vertex_index) & 1;
    let pos = vec4<f32>(
        f32(tmp1) * 4.0 - 1.0,
        f32(tmp2) * 4.0 - 1.0,
        1.0,
        1.0
    );

    let view = camera.view;
    // transposition = inversion for this orthonormal matrix
    let inv_model_view = transpose(mat3x3<f32>(view.x.xyz, view.y.xyz, view.z.xyz));
    let unprojected = camera.proj_inv * pos;

    var result: SkyOutput;
    result.uv = inv_model_view * unprojected.xyz;
    result.position = pos;
    return result;
}


@fragment
fn fs_sky(vertex: SkyOutput) -> @location(0) vec4<f32> {
    let color = textureSample(r_texture, r_sampler, vertex.uv);
    return vec4<f32>(color.rgb * 0.1, color.a);
}