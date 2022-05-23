use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use cgmath::Rotation3;
use wgpu::{Device, RenderPass, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use crate::{Camera, geo_gen, MULTI_SAMPLE, PRIMITIVE, RenderGroup, texture, uniform_desc};
use crate::geo_gen::GeoObj;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    // pos_dir[3] == 1? position: direction
    pub pos_dir: [f32; 4],
    pub color: [f32; 3],
    padding_0: f32,
    pub diffuse_strength: f32,
    pub ambient_strength: f32,
    pub specular_strength: f32,
    padding_1: f32,
    // constant, linear, quadratic
    // point_clq[4] == 0? no_attenuation: attenuation
    point_clq: [f32; 4],
    // cutoff_inner_outer_eps[4] == 0? no_cutoff: cutoff
    cutoff_inner_outer_eps: [f32; 4]
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            pos_dir: [40., 10., -10.0, 1.],
            color: [1.0; 3],
            padding_0: 0.,
            diffuse_strength: 1.0,
            ambient_strength: 0.1,
            specular_strength: 0.3,
            padding_1: 0.,
            point_clq: [1., 0.045, 0.0075, 1.],
            cutoff_inner_outer_eps: [0.; 4]
        }
    }
}

pub struct LightRenderGroup {
    light_uniforms: Vec<LightUniform>,
    buffer: wgpu::Buffer,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    obj: GeoObj
}



impl LightRenderGroup {
    pub fn new(device: &Device, light_uniforms: Vec<LightUniform>, shader: ShaderModule, camera: &Camera, config: &SurfaceConfiguration) -> Rc<RefCell<Self>> {
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&light_uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        );
        let light_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {read_only: true},
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Light Storage BindGroupLayout"),
        });

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
        Rc::new(RefCell::new(Self {
            light_uniforms,
            buffer,
            light_bind_group_layout,
            light_bind_group,
            light_render_pipeline,
            obj: geo_gen::create_cube(10.0, device)
        }))
    }

    pub fn update_light(&mut self, dt: Duration, queue: &wgpu::Queue) {
        let rotation: cgmath::Matrix4<f32> = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(-100. * dt.as_secs_f32())).into();
        let pos = cgmath::Vector4::from(self.light_uniforms[0].pos_dir);
        self.light_uniforms[0].pos_dir = (rotation * pos).into();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.light_uniforms));
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
