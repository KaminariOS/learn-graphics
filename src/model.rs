use std::cell::RefCell;
use std::default::Default;
use std::ops::Range;
use std::rc::Rc;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, RenderPass, RenderPipeline, SurfaceConfiguration,
};

use crate::geo_gen::Vertex;
use crate::{
    texture, uniform_desc, world_space, Camera, LightRenderGroup, RenderGroup, MULTI_SAMPLE,
    PRIMITIVE,
};

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
    pub uniform_bind_group: MaterialGroup,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub ambient: [f32; 3],
    _padding0: f32,
    pub diffuse: [f32; 3],
    _padding1: f32,
    pub specular: [f32; 3],
    _padding2: f32,
    pub shininess: f32,
    _padding3: [f32; 3],
}

impl Default for MaterialUniform {
    fn default() -> Self {
        Self {
            ambient: [1.0; 3],
            diffuse: [1.0; 3],
            specular: [1.0; 3],
            shininess: 32.0,
            _padding0: 0.,
            _padding1: 0.,
            _padding2: 0.,
            _padding3: [0.; 3],
        }
    }
}

impl MaterialUniform {
    pub fn new(ambient: [f32; 3], diffuse: [f32; 3], specular: [f32; 3], shininess: f32) -> Self {
        Self {
            ambient,
            diffuse,
            specular,
            shininess,
            ..Default::default()
        }
    }
    pub fn create_buffer_and_bindgroup(self, device: &Device) -> MaterialGroup {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout =
            device.create_bind_group_layout(&uniform_desc("MaterialUniform layout"));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("material bind_group"),
        });
        MaterialGroup {
            uniform: self,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }
}

pub struct MaterialGroup {
    uniform: MaterialUniform,
    buffer: Buffer,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub texture_bind_group_layout: BindGroupLayout,
}

pub(crate) struct ModelRenderGroup {
    model: Model,
    instances: world_space::Instances,
    render_pipeline: RenderPipeline,
}

impl ModelRenderGroup {
    pub fn new(
        model: Model,
        instances: world_space::Instances,
        device: &Device,
        camera: &Camera,
        config: &SurfaceConfiguration,
        light_render_group: &LightRenderGroup,
    ) -> Rc<RefCell<Self>> {
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Model Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera.camera_bind_group_layout,
                    &light_render_group.light_bind_group_layout,
                    &model.texture_bind_group_layout,
                    &model.materials[0].uniform_bind_group.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Model Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[world_space::desc(), Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: PRIMITIVE,
            depth_stencil: texture::Texture::create_depth_state(),
            multisample: MULTI_SAMPLE,
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });
        Rc::new(RefCell::new(Self {
            model,
            instances,
            render_pipeline,
        }))
    }

    fn draw_mesh_instanced<'a, 'b: 'a>(
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        render_pass: &mut RenderPass<'a>,
    ) {
        render_pass.set_vertex_buffer(1, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(2, &material.bind_group, &[]);
        render_pass.set_bind_group(3, &material.uniform_bind_group.bind_group, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

impl RenderGroup for ModelRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.instances.instance_buffer.slice(..));
        for mesh in &self.model.meshes {
            let material = &self.model.materials[mesh.material];
            Self::draw_mesh_instanced(
                mesh,
                material,
                self.instances.get_instance_range(),
                render_pass,
            );
        }
    }
}
