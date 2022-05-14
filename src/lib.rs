use std::iter;
use cgmath::{One, Quaternion};

mod camera;
use camera::Camera;

mod geo_gen;
use geo_gen::Entity;

mod texture;
mod world_space;
mod model;

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::RenderPipeline;
use crate::camera::{CameraController, CameraView, Projection};
use crate::geo_gen::{prepare_tex, RenderGroup};
use crate::texture::Texture;
use crate::world_space::{Instances, InstanceTransform};


#[cfg(target_arch = "wasm32")]
const SAMPLE_COUNT: u32 = 1;
#[cfg(not(target_arch = "wasm32"))]
const SAMPLE_COUNT: u32 = 4;


struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    // NEW!

    tex_view: wgpu::TextureView,
    frame_count: usize,
    camera: Camera,
    camera_controller: CameraController,
    mouse_pressed: bool,
    entity: Entity,
    rp_tex: RenderPipeline,
    depth_texture: Texture,
    render_groups: Vec<RenderGroup>
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let camera = Camera::new(
            CameraView::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0)),
            Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 200.0),
            &device);

        let obj = geo_gen::create_plane(80.0);
        let entity = Entity::new(&device, &queue, obj, include_bytes!("albedo.png"));
        let render_group = {
            let obj = geo_gen::create_cube(10.0);
            let entity_cube = Entity::new(&device, &queue, obj, include_bytes!("img.png"));
            let instances = Instances::new(vec![InstanceTransform {
                position: cgmath::Vector3::new(0.0, 0.0, -10.0),
                rotation: Quaternion::one(),
            }], &device);
            let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("geo.wgsl").into()),
            });
            RenderGroup::new(&device, &camera, entity_cube, instances, shader, &config)
        };
        let render_groups = vec![render_group];

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let tex_view = create_multisampled_framebuffer(&device, &config);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let rp_tex = prepare_tex(&device, &config, &camera, &entity);
        Self {
            surface,
            device,
            queue,
            config,
            size,
            tex_view,
            frame_count: 0,
            camera,
            camera_controller,
            mouse_pressed: false,
            entity,
            rp_tex,
            depth_texture,
            render_groups
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.camera.projection.resize(new_size.width, new_size.height);
            self.surface.configure(&self.device, &self.config);
            self.tex_view = create_multisampled_framebuffer(&self.device, &self.config);
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

        }
    }

    fn input(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                KeyboardInput {
                    virtual_keycode: Some(key),
                    state,
                    ..
                },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = true;
                window.set_cursor_grab(true).ok();
                window.set_cursor_visible(false);
                true
            }
            _ => false,
        }
    }


    fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera.view, dt);
        self.camera.update_camera(&self.queue);
        // Update the light
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.frame_count += 1;
        // println!("frame count: {}", self.frame_count);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let tex_view = &self.tex_view;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: if SAMPLE_COUNT == 1 { &view } else { tex_view },
                    resolve_target: Some(&view).filter(|_| SAMPLE_COUNT != 1),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.entity.texture_bind_group, &[]);

            render_pass.set_pipeline(&self.rp_tex);
            render_pass.set_vertex_buffer(0, self.entity.vertex_buffer.slice(..));
            render_pass.set_index_buffer( self.entity.index_buffer.slice(..), self.entity.index_format);
            render_pass.draw_indexed(0..self.entity.obj.index_data.len() as u32, 0, 0..1);
            self.render_groups.iter().for_each(|x| x.render(&mut render_pass));
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::LogicalSize;

        use winit::platform::web::WindowExtWebSys;
        let win = web_sys::window().expect("Failed to get window object");
        fn get_size(val: Result<JsValue, JsValue>) -> f64 {
            val.ok().and_then(|x| x.as_f64()).unwrap()
        }
        let (width, height) = (get_size(win.inner_width()), get_size(win.inner_height()));
        log::info!("Window size: {:?} {:?}", width, height);
        window.set_inner_size(LogicalSize::new(height, height));
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());

                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await;

    let mut last_render_time = instant::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            // NEW!
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            // UPDATED!
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !state.input(event, &window) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                     => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => {
                        window.set_cursor_grab(false).ok();
                        window.set_cursor_visible(true);
                        state.mouse_pressed = false;
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            // UPDATED!
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = instant::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt);
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        }
    });
}

