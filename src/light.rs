use std::time::Duration;
use cgmath::{Matrix3, Rotation3};
use wgpu::{Device, RenderPass, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use crate::{Camera, geo_gen, MULTI_SAMPLE, PRIMITIVE, RenderGroup, texture, uniform_desc};
use crate::geo_gen::GeoObj;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub(crate) position: [f32; 4],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) color: [f32; 4],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub ambient_strength: f32,
    _padding1: [f32; 3],
    pub specular_strength: f32,
    _padding2: [f32; 3],
}

impl Default for LightUniform {
    fn default() -> Self {
       Self {
           position: [40., 10., -10.0, 1.],
           color: [1., 1., 1., 1.],
           ambient_strength: 0.1,
           _padding1: [0.; 3],
           specular_strength: 1.0,
           _padding2: [0.;3]
       }
    }
}

pub struct LightRenderGroup {
    light_uniform: LightUniform,
    buffer: wgpu::Buffer,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    obj: GeoObj
}

impl LightRenderGroup {
    pub fn new(device: &Device, light_uniform: LightUniform, shader: ShaderModule, camera: &Camera, config: &SurfaceConfiguration) -> Self {
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let light_bind_group_layout =
            device.create_bind_group_layout(&uniform_desc("LightUniform BindGroupLayout"));

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Light Pipeline Layout"),
            bind_group_layouts: &[&camera.camera_bind_group_layout, &light_bind_group_layout],
            push_constant_ranges: &[],
        });
        let light_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Light Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[geo_gen::Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: PRIMITIVE,
            depth_stencil: texture::Texture::create_depth_state(),
            multisample: MULTI_SAMPLE,
            multiview: None
        });
        Self {
            light_uniform,
            buffer,
            light_bind_group_layout,
            light_bind_group,
            light_render_pipeline,
            obj: geo_gen::create_cube(10.0, device)
        }
    }

    pub fn update_light(&mut self, dt: Duration, queue: &wgpu::Queue) {
        let rotation: cgmath::Matrix4<f32> = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(-100. * dt.as_secs_f32())).into();
        let pos = cgmath::Vector4::from(self.light_uniform.position);
        self.light_uniform.position = (rotation * pos).into();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
    }

}

impl RenderGroup for LightRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.set_bind_group(1, &self.light_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.obj.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.obj.index_buffer.slice(..), GeoObj::INDEX_FORMAT);
        render_pass.draw_indexed(self.obj.get_index_range(), 0, 0..1);
    }
}