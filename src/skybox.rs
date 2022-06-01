use crate::{resources, texture, Camera, RenderGroup, MULTI_SAMPLE};
use image::{DynamicImage, GenericImageView};
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use wgpu::{
    BindGroup, Device, Queue, RenderPass, RenderPipeline, SurfaceConfiguration, Texture,
    TextureDimension,
};

pub struct SkyboxRenderGroup {
    sky_pipeline: RenderPipeline,
    bind_group: BindGroup,
}

impl RenderGroup for SkyboxRenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>, shadow_pass: bool) {
        if shadow_pass {
            return;
        }
        render_pass.set_bind_group(1, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.sky_pipeline);
        render_pass.draw(0..3, 0..1);
    }
}

pub async fn create(
    device: &Device,
    config: &SurfaceConfiguration,
    queue: &Queue,
    camera: &Camera,
) -> Rc<RefCell<SkyboxRenderGroup>> {
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("skybox.wgsl").into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
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
    });
    let tex = create_cubemap(device, queue).await;
    let texture_view = tex.create_view(&wgpu::TextureViewDescriptor {
        label: Some("cubemap view"),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..wgpu::TextureViewDescriptor::default()
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&camera.camera_bind_group_layout, &bind_group_layout],
        push_constant_ranges: &[],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });
    let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sky"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_sky",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_sky",
            targets: &[config.format.into()],
        }),
        primitive: wgpu::PrimitiveState {
            front_face: wgpu::FrontFace::Cw,
            ..Default::default()
        },
        depth_stencil: texture::Texture::create_depth_state(),
        multisample: MULTI_SAMPLE,
        multiview: None,
    });
    Rc::new(RefCell::new(SkyboxRenderGroup {
        sky_pipeline,
        bind_group,
    }))
}

async fn load_cubemap(dir: &str, ext: &str) -> Vec<DynamicImage> {
    let faces = ["posx", "negx", "posy", "negy", "posz", "negz"];
    let mut vec = vec![];
    for face in faces {
        let filename = face.to_owned() + ext;
        let path = std::path::Path::new(dir).join(filename);
        let bytes = resources::load_binary(path.to_str().unwrap())
            .await
            .unwrap();
        vec.push(image::load_from_memory(&bytes).unwrap());
    }
    vec
}

async fn create_cubemap(device: &Device, queue: &Queue) -> Texture {
    // let images = load_cubemap("Yokohama", ".jpg").await;
    let images = load_cubemap("skype", ".png").await;
    let (width, height) = images[0].dimensions();
    let total =
        images
            .into_iter()
            .map(|x| x.to_rgba8().into_raw())
            .fold(vec![], |mut acc, next| {
                acc.extend(next);
                acc
            });
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 6,
    };
    let layer_size = wgpu::Extent3d {
        depth_or_array_layers: 1,
        ..size
    };
    let max_mips = layer_size.max_mips(wgpu::TextureDimension::D2);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Cube"),
        size,
        mip_level_count: max_mips,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: texture::TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        &total,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: NonZeroU32::new(4 * width),
            rows_per_image: NonZeroU32::new(height),
        },
        size,
    );
    tex
}
