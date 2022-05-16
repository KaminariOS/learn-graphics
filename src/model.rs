use std::ops::Range;
use wgpu::{BindGroupLayout, Device, RenderPass, RenderPipeline, ShaderModule, SurfaceConfiguration};

use crate::{Camera, Instances, LightRenderGroup, MULTI_SAMPLE, PRIMITIVE, RenderGroup, texture, world_space};
use crate::geo_gen::Vertex;


pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn create_texture_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
    }
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
    pub texture_bind_group_layout: BindGroupLayout
}

pub(crate) struct ModelRenderGroup {
    model: Model,
    instances: world_space::Instances,
    render_pipeline: RenderPipeline
}

impl ModelRenderGroup {
    pub fn new(model: Model, instances: world_space::Instances, device: &Device, camera: &Camera, shader: ShaderModule, config: &SurfaceConfiguration, light_render_group: &LightRenderGroup) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera.camera_bind_group_layout, &light_render_group.light_bind_group_layout, &model.texture_bind_group_layout],
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
        Self {
            model,
            instances,
            render_pipeline
        }
    }

    fn draw_mesh_instanced<'a, 'b: 'a>(
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        render_pass: &mut RenderPass<'a>
    ) {
        render_pass.set_vertex_buffer(1, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(2, &material.bind_group, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}


impl RenderGroup for ModelRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.instances.instance_buffer.slice(..));
        for mesh in &self.model.meshes {
            let material = &self.model.materials[mesh.material];
            Self::draw_mesh_instanced(mesh, material, self.instances.get_instance_range(), render_pass);
        }
    }
}

