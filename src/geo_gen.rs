use cgmath::SquareMatrix;
use wgpu::util::DeviceExt;

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

struct GeoObj {
    vertex_data: Vec<Vertex>,
    index_data: Vec<u16>
}

fn create_plane(size: f32) -> GeoObj {
    let vertex_data = vec![
        Vertex::new([size, -size, 0.0], [1.0, 0.0]),
        Vertex::new([size, size, 0.0], [1.0, 1.0]),
        Vertex::new([-size, -size, 0.0], [0.0, 0.0]),
        Vertex::new([-size, size, 0.0], [0.0, 1.0]),
    ];
    let index_data = vec![0, 1, 2, 2, 1, 3];
    GeoObj {
        vertex_data,
        index_data
    }
}

struct Entity {
    obj: GeoObj,
    world_transform: cgmath::Matrix3<f32>,
    rotation_speed: f32,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
}

impl Entity {
    fn new(device: &wgpu::Device) -> Self {
        let obj= create_plane(8.0);
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
        Self {
            obj,
            world_transform: cgmath::Matrix3::identity(),
            rotation_speed: 0.0,
            vertex_buffer,
            index_buffer,
            index_format: wgpu::IndexFormat::Uint16
        }
    }
}