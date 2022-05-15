use std::ops::Range;
use wgpu::{Device, Queue, RenderPipeline, ShaderModule, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use crate::{SAMPLE_COUNT, world_space};
use crate::{Camera, texture};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn new(position: [f32; 3], tex_coords: [f32; 2]) -> Self {
        Vertex {position, tex_coords}
    }
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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
            ],
        }
    }
}

pub struct GeoObj {
    vertex_data: Vec<Vertex>,
    pub(crate) index_data: Vec<u16>,
}

pub struct Entity {
    pub(crate) obj: GeoObj,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_format: wgpu::IndexFormat,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group: wgpu::BindGroup
}

impl Entity {
    pub(crate) fn new(device: &Device, queue: &Queue, obj: GeoObj, diffuse_bytes: &[u8]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&obj.vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&obj.index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
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
            vertex_buffer,
            index_buffer,
            index_format: wgpu::IndexFormat::Uint16,
            texture_bind_group_layout,
            texture_bind_group
        }
    }

    fn get_index_range(&self) -> Range<u32> {
        0..self.obj.index_data.len() as u32
    }
}

pub struct RenderGroup {
    pub(crate) entity: Entity,
    instances: world_space::Instances,
    pub(crate) render_pipeline: RenderPipeline,
}

impl RenderGroup {
    pub fn new(device: &wgpu::Device, camera: &Camera, entity: Entity, instances: world_space::Instances, shader: ShaderModule, config: &SurfaceConfiguration) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera.camera_bind_group_layout, &entity.texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
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

    pub fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(1, &self.entity.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.entity.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.entity.index_buffer.slice(..), self.entity.index_format);
        render_pass.draw_indexed(self.entity.get_index_range(), 0, self.instances.get_instance_range());
    }
}

pub fn create_square(height: f32, width: f32) -> GeoObj {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let vertex_data = vec![
        Vertex::new([half_width, half_height, 0.0], [1.0, 1.0]),
        Vertex::new([-half_width, half_height, 0.0], [0.0, 1.0]),
        Vertex::new([-half_width, -half_height, 0.0], [0.0, 0.0]),
        Vertex::new([half_width, -half_height, 0.0], [1.0, 0.0]),
    ];
    let index_data = vec![0, 1, 2, 2, 3, 0];
    GeoObj {
        vertex_data,
        index_data,
    }
}

pub fn create_floor(height: f32, width: f32) -> GeoObj {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let mul = 10.0;
    let vertex_data = vec![
        Vertex::new([half_width, half_height, 0.0], [mul, mul]),
        Vertex::new([-half_width, half_height, 0.0], [0.0, mul]),
        Vertex::new([-half_width, -half_height, 0.0], [0.0, 0.0]),
        Vertex::new([half_width, -half_height, 0.0], [mul, 0.0]),
    ];
    let index_data = vec![0, 1, 2, 2, 3, 0];
    GeoObj {
        vertex_data,
        index_data,
    }
}
