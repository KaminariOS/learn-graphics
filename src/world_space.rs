use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, VertexAttribute};

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
    static ATRIBUTES: &[VertexAttribute; 7] = &wgpu::vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x3,
        10 => Float32x3,
        11 => Float32x3,
        ];
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
        // We need to switch from using a step mode of Vertex to Instance
        // This means that our shaders will only change to use the next
        // instance when the shader starts processing a new instance
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: ATRIBUTES
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
