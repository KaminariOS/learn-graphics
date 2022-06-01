use crate::geo_gen::GeoObj;
use crate::{geo_gen, texture, Camera, Projection, RenderGroup, State, MULTI_SAMPLE, PRIMITIVE};
use cgmath::{Angle, Deg, Matrix4, Point3, Rotation3, SquareMatrix, Vector3};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, Device, RenderPass, SurfaceConfiguration};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    // position[3] == 1? point: directional
    pub position: [f32; 3],
    pub _padding_2: f32,
    pub direction: [f32; 3],
    pub _padding_3: f32,
    pub color: [f32; 4],
    pub diffuse_strength: f32,
    pub ambient_strength: f32,
    pub specular_strength: f32,
    pub _padding_1: f32,
    // constant, linear, quadratic
    // point_clq[4] == 0? no_attenuation: attenuation
    pub point_clq: [f32; 4],
    // cutoff_inner_outer_eps[4] == 0? no_cutoff: cutoff
    pub cutoff_inner_outer_eps: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            position: [40., 20., -40.0],
            direction: [0.; 3],
            _padding_2: 0.,
            _padding_3: 0.,
            color: [1.0, 1.0, 1.0, 0.],
            diffuse_strength: 1.0,
            ambient_strength: 0.1,
            specular_strength: 0.3,
            _padding_1: 0.,
            point_clq: [1., 0.045, 0.0075, 1.],
            cutoff_inner_outer_eps: [0.; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
}

impl LightUniform {
    pub fn calc_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(
            Point3::from(self.position),
            -Vector3::from(self.direction),
            Vector3::unit_y(),
        )
    }
    fn calc_view_proj(&mut self, config: &SurfaceConfiguration) {
        self.view_proj =
            (Projection::new(config.width, config.height, cgmath::Deg(45.0), 1.0, 300.0)
                .calc_matrix()
                * self.calc_view_matrix())
            .into();
    }

    pub fn build_light(mut light: Self, config: &SurfaceConfiguration) -> Self {
        light.calc_view_proj(config);
        light
    }
}

pub fn cal_cutoff(inner: f32, outer: f32) -> [f32; 4] {
    let inner = Deg(inner).cos();
    let outer = Deg(outer).cos();
    assert!(inner > outer);
    [inner, outer, inner - outer, 1.]
}

pub struct LightRenderGroup {
    pub light_uniforms: Vec<LightUniform>,
    buffer: wgpu::Buffer,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    pub light_render_triplets: Vec<(Buffer, BindGroup, GeoObj)>,
}

impl LightRenderGroup {
    pub fn new(
        device: &Device,
        light_uniforms_and_objs: Vec<(LightUniform, GeoObj)>,
        camera: &Camera,
        config: &SurfaceConfiguration,
    ) -> Rc<RefCell<Self>> {
        let (light_uniforms, objs): (Vec<LightUniform>, Vec<GeoObj>) =
            light_uniforms_and_objs.into_iter().unzip();
        let light_uniforms: Vec<_> = light_uniforms
            .into_iter()
            .map(|x| LightUniform::build_light(x, config))
            .collect();
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Light Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
        });
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&light_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Light Storage BindGroupLayout"),
            });
        let light_render_triplets: Vec<_> = light_uniforms
            .iter()
            .zip(objs)
            .map(|(light_uniform, obj)| {
                let buffer_per_light =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Light VB"),
                        contents: bytemuck::cast_slice(&[*light_uniform]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });
                let bind_group_per_light = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &light_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer_per_light.as_entire_binding(),
                    }],
                    label: None,
                });
                (buffer_per_light, bind_group_per_light, obj)
            })
            .collect();
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera.camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
        let light_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                multiview: None,
            });
        Rc::new(RefCell::new(Self {
            light_uniforms,
            buffer,
            light_bind_group_layout,
            light_bind_group,
            light_render_pipeline,
            light_render_triplets,
        }))
    }

    pub fn update_light(&mut self, dt: Duration, state: &State) {
        let rotation: cgmath::Matrix3<f32> = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_y(),
            cgmath::Deg(-100. * dt.as_secs_f32()),
        )
        .into();
        for (i, uniform) in self.light_uniforms.iter_mut().enumerate() {
            *uniform = LightUniform::build_light(*uniform, &state.config);
            if i == 0 {
                let pos = cgmath::Vector3::from(uniform.position);
                uniform.position = (rotation * pos).into();
                uniform.direction = uniform.position;
            } else if i == 1 {
                let dir = state.camera.view.get_dir();
                uniform.position = (state.camera.view.position + dir * 10.0).into();
                uniform.direction = (-dir).into();
            }
        }
        for ((buffer, _, _), uniform) in self.light_render_triplets.iter().zip(&self.light_uniforms)
        {
            state
                .queue
                .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.light_uniforms));
            state
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&[*uniform]));
        }
    }
}

impl RenderGroup for LightRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>, shadow_pass: bool) {
        if shadow_pass {
            return;
        }
        render_pass.set_pipeline(&self.light_render_pipeline);
        for (i, (_, bind_group_per_light, obj)) in self.light_render_triplets.iter().enumerate() {
            if self.light_uniforms[i].color[3] == 0. {
                continue;
            }
            render_pass.set_bind_group(1, bind_group_per_light, &[]);
            render_pass.set_vertex_buffer(0, obj.vertex_buffer.slice(..));
            render_pass.set_index_buffer(obj.index_buffer.slice(..), GeoObj::INDEX_FORMAT);
            render_pass.draw_indexed(obj.get_index_range(), 0, 0..1);
        }
        render_pass.set_bind_group(1, &self.light_bind_group, &[]);
    }
}
