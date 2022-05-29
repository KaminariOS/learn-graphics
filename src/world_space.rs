use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device};

pub struct InstanceTransform {
    pub(crate) position: cgmath::Vector3<f32>,
    pub(crate) rotation: cgmath::Quaternion<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
}

impl InstanceTransform {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}

pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    use std::mem;
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
        // We need to switch from using a step mode of Vertex to Instance
        // This means that our shaders will only change to use the next
        // instance when the shader starts processing a new instance
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                shader_location: 5,
                format: wgpu::VertexFormat::Float32x4,
            },
            // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
            // for each vec4. We'll have to reassemble the mat4 in
            // the shader.
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                shader_location: 6,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                shader_location: 7,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                shader_location: 8,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                shader_location: 9,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                shader_location: 10,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                shader_location: 11,
                format: wgpu::VertexFormat::Float32x3,
            },
        ],
    }
}

pub struct Instances {
    pub instance_transforms: Vec<InstanceTransform>,
    // instances_raw: Vec<InstanceRaw>,
    pub instance_buffer: wgpu::Buffer,
}

impl Instances {
    fn get_raw_and_buffer(instance_transforms: &Vec<InstanceTransform>, device: &Device) -> Buffer {
        let instances_raw: Vec<InstanceRaw> = instance_transforms
            .iter()
            .map(InstanceTransform::to_raw)
            .collect();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances_raw),
            usage: wgpu::BufferUsages::VERTEX,
        });
        instance_buffer
    }
    pub(crate) fn new(instance_transforms: Vec<InstanceTransform>, device: &Device) -> Self {
        let instance_buffer = Self::get_raw_and_buffer(&instance_transforms, device);
        Self {
            instance_transforms,
            instance_buffer,
        }
    }

    pub fn get_instance_range(&self) -> Range<u32> {
        0..self.instance_transforms.len() as u32
    }
}
