use std::ops::Range;
use wgpu::{BindGroupLayout, Device, IndexFormat, Queue, RenderPipeline, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use crate::{Instances, LightRenderGroup, MULTI_SAMPLE, PRIMITIVE, RenderGroup, world_space};
use crate::{Camera, texture};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) tex_coords: [f32; 2],
    pub(crate) normal: [f32; 3]
}

impl Vertex {
    fn new(position: [f32; 3], tex_coords: [f32; 2], normal: [f32; 3]) -> Self {
        Vertex {position, tex_coords, normal}
    }
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct GeoObj {
    vertex_data: Vec<Vertex>,
    pub(crate) index_data: Vec<u16>,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
}

impl GeoObj {
    pub const INDEX_FORMAT: IndexFormat = IndexFormat::Uint16;
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u16>, device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_data,
            index_data,
            vertex_buffer,
            index_buffer
        }
    }
    pub(crate) fn get_index_range(&self) -> Range<u32> {
        0..self.index_data.len() as u32
    }
}

pub struct Entity {
    pub(crate) obj: GeoObj,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group: wgpu::BindGroup
}

impl Entity {
    pub(crate) fn new(device: &Device, queue: &Queue, obj: GeoObj, diffuse_bytes: &[u8]) -> Self {
        let diffuse_texture = texture::Texture::from_bytes(device, queue, diffuse_bytes, "Todo").unwrap();
        let texture_bind_group_layout =
            device.create_bind_group_layout(&texture::Texture::desc());
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        Self {
            obj,
            texture_bind_group_layout,
            texture_bind_group
        }
    }


}

pub struct GeoRenderGroup {
    pub(crate) entity: Entity,
    instances: world_space::Instances,
    pub(crate) render_pipeline: RenderPipeline,
}

impl GeoRenderGroup {
    pub fn new(device: &wgpu::Device, camera: &Camera, entity: Entity, instances: world_space::Instances, shader: ShaderModule, config: &SurfaceConfiguration, light_render_group: &LightRenderGroup) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera.camera_bind_group_layout, &light_render_group.light_bind_group_layout, &entity.texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Entity Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), world_space::desc()],
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
            entity,
            instances,
            render_pipeline
        }
    }


}

impl RenderGroup for GeoRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(2, &self.entity.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.entity.obj.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.entity.obj.index_buffer.slice(..), GeoObj::INDEX_FORMAT);
        render_pass.draw_indexed(self.entity.obj.get_index_range(), 0, self.instances.get_instance_range());
    }
}
pub fn create_square(height: f32, width: f32, device: &Device) -> GeoObj {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let vertex_data = vec![
        Vertex::new([half_width, half_height, 0.0], [1.0, 1.0], [0., 0., 1.]),
        Vertex::new([-half_width, half_height, 0.0], [0.0, 1.0], [0., 0., 1.]),
        Vertex::new([-half_width, -half_height, 0.0], [0.0, 0.0], [0., 0., 1.]),
        Vertex::new([half_width, -half_height, 0.0], [1.0, 0.0], [0., 0., 1.]),
    ];
    let index_data = vec![0, 1, 2, 2, 3, 0];
    GeoObj::new(
        vertex_data,
        index_data,
        device
    )
}

pub fn create_floor(height: f32, width: f32, device: &Device) -> GeoObj {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let mul = 10.0;
    let vertex_data = vec![
        Vertex::new([half_width, half_height, 0.0], [mul, mul], [0., 0., 1.]),
        Vertex::new([-half_width, half_height, 0.0], [0.0, mul], [0., 0., 1.]),
        Vertex::new([-half_width, -half_height, 0.0], [0.0, 0.0], [0., 0., 1.]),
        Vertex::new([half_width, -half_height, 0.0], [mul, 0.0], [0., 0., 1.]),
    ];
    let index_data = vec![0, 1, 2, 2, 3, 0];
    GeoObj::new(
        vertex_data,
        index_data,
        device
    )
}

pub fn create_cube(size: f32, device: &Device) -> GeoObj {
   let half = size / 2.;
    let vertex_data = vec![
       Vertex::new([-half, -half, half], [0., 0.], [0., 0., 1.]),
       Vertex::new([half, -half, half], [0., 0.],[0., 0., 1.]),
       Vertex::new([half, half, half], [0., 0.], [0., 0., 1.]),
       Vertex::new([-half, half, half], [0., 0.], [0., 0., 1.]),
       Vertex::new([-half, -half, -half], [0., 0.], [0., 0., 1.]),
       Vertex::new([half, -half, -half], [0., 0.], [0., 0., 1.]),
       Vertex::new([half, half, -half], [0., 0.], [0., 0., 1.]),
       Vertex::new([-half, half, -half], [0., 0.],[0., 0., 1.]),
    ];
    let index_data = vec![
        //Front
        0, 1, 2,
        2, 3, 0,

        // Back
        4, 7, 6,
        6, 5, 4,

        // Top
        3, 2, 6,
        6, 7, 3,

        // Bottom
        0, 4, 5,
        5, 1, 0,

        // Left
        0, 3, 7,
        7, 4, 0,

        // Right
        1, 5, 6,
        6, 2, 1
    ];
    GeoObj::new(
        vertex_data,
        index_data,
        device
    )
}