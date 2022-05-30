use cgmath::prelude::*;
use cgmath::{Quaternion, Vector3};
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::time::Duration;

mod camera;
use camera::Camera;

mod geo_gen;
use geo_gen::Entity;

mod light;
mod model;
mod resources;
mod texture;
mod world_space;
mod skybox;

use model::ModelRenderGroup;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::camera::{CameraController, CameraView, Projection};
use crate::geo_gen::{create_sphere, GeoRenderGroup};
use crate::light::{LightRenderGroup, LightUniform};
use crate::texture::Texture;
use crate::world_space::{InstanceTransform, Instances};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
const SAMPLE_COUNT: u32 = 1;
#[cfg(not(target_arch = "wasm32"))]
const SAMPLE_COUNT: u32 = 4;

const TEXTURE_SAMPLE_COUNT: u32 = 1;

const MULTI_SAMPLE: wgpu::MultisampleState = wgpu::MultisampleState {
    count: SAMPLE_COUNT,
    mask: !0,
    alpha_to_coverage_enabled: false,
};

const FLOOR_HEIGHT: f32 = -10.0;
const PRIMITIVE: wgpu::PrimitiveState = wgpu::PrimitiveState {
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
};

pub trait RenderGroup {
    fn render<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>);
}

static UNIFORM_BIND_GROUP_LAYOUT_ENTRY: [wgpu::BindGroupLayoutEntry; 1] =
    [wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }];

fn uniform_desc(label_str: &str) -> wgpu::BindGroupLayoutDescriptor {
    wgpu::BindGroupLayoutDescriptor {
        entries: &UNIFORM_BIND_GROUP_LAYOUT_ENTRY,
        label: Some(label_str),
    }
}

pub struct State {
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
    depth_texture: Texture,
    render_groups: Vec<Rc<RefCell<dyn RenderGroup>>>,
    light_render_group: Rc<RefCell<LightRenderGroup>>,
    render_group_sphere: Rc<RefCell<GeoRenderGroup>>,
    total_duration: Duration,
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
                        {
                            let mut limit = wgpu::Limits::downlevel_webgl2_defaults();
                            limit.max_texture_dimension_2d = 4096;
                            limit
                        }
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
            Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 800.0),
            &device,
        );

        let light_render_group = {
            LightRenderGroup::new(
                &device,
                vec![
                    (
                        LightUniform {
                            color: [1., 1., 1., 1.],
                            // point_clq: [0.0; 4],
                            ..Default::default()
                        },
                        geo_gen::create_cube(10.0, &device),
                    ),
                    (
                        LightUniform {
                            cutoff_inner_outer_eps: light::cal_cutoff(4.0, 30.0),
                            ambient_strength: 0.01,
                            ..Default::default()
                        },
                        geo_gen::create_sphere(10., 20, 20, &device),
                    ),
                ],
                &camera,
                &config,
            )
        };
        let render_group = {
            let height = 26.0;
            let half_height = height / 2.0;
            let obj = geo_gen::create_square(height, 40.0, &device);
            let entity_cube = Entity::new(&device, &queue, obj, include_bytes!("asuka.png"), 1);
            let instances = Instances::new(
                vec![
                    InstanceTransform {
                        position: Vector3::new(0.0, half_height + FLOOR_HEIGHT, -40.0),
                        rotation: Quaternion::one(),
                    },
                    // InstanceTransform {
                    //     position: Vector3::new(10.0, half_height + FLOOR_HEIGHT, 0.0),
                    //     rotation: Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(-90.0))
                    // }
                ],
                &device,
            );
            GeoRenderGroup::new(
                &device,
                &camera,
                entity_cube,
                instances,
                &config,
                &light_render_group.borrow(),
            )
        };
        let render_group_floor = {
            let obj = geo_gen::create_floor(2800.0, 2800.0, &device);
            let entity_cube = Entity::new(&device, &queue, obj, include_bytes!("albedo.png"), 11);
            let instances = Instances::new(
                vec![InstanceTransform {
                    position: Vector3::new(00.0, FLOOR_HEIGHT, 0.0),
                    rotation: Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_x(),
                        cgmath::Deg(-90.0),
                    ),
                }],
                &device,
            );
            GeoRenderGroup::new(
                &device,
                &camera,
                entity_cube,
                instances,
                &config,
                &light_render_group.borrow(),
            )
        };
        let render_group_sphere = {
            let obj = geo_gen::create_sphere(10.0, 3, 2, &device);
            let entity_cube =
                Entity::new(&device, &queue, obj, include_bytes!("texture_test.png"), 1);
            let instances = Instances::new(
                vec![InstanceTransform {
                    position: Vector3::new(60.0, 5.0, -15.0),
                    rotation: Quaternion::one(),
                }],
                &device,
            );
            GeoRenderGroup::new(
                &device,
                &camera,
                entity_cube,
                instances,
                &config,
                &light_render_group.borrow(),
            )
        };

        let model_render_group = {
            log::warn!("Load model");
            let obj_model = resources::load_model("girl.obj", &device, &queue, 40.0)
                .await
                .unwrap();
            let instances = Instances::new(
                vec![InstanceTransform {
                    position: Vector3::new(-60.0, -11.0, 0.0),
                    rotation: Quaternion::one(),
                }],
                &device,
            );
            ModelRenderGroup::new(
                obj_model,
                instances,
                &device,
                &camera,
                &config,
                &light_render_group.borrow(),
            )
        };
        let sword_model_render_group = {
            log::warn!("Load model");
            let obj_model = resources::load_model("arto.obj", &device, &queue, 1.0)
                .await
                .unwrap();
            let instances = Instances::new(
                vec![InstanceTransform {
                    position: Vector3::new(-0.0, -10.0, 0.0),
                    rotation: Quaternion::one(),
                }],
                &device,
            );
            ModelRenderGroup::new(
                obj_model,
                instances,
                &device,
                &camera,
                &config,
                &light_render_group.borrow(),
            )
        };
        let skybox = skybox::create(&device, &config, &queue, &camera).await;
        let render_groups: Vec<Rc<RefCell<dyn RenderGroup>>> = vec![
            skybox,
            light_render_group.clone(),
            render_group,
            render_group_floor,
            model_render_group,
            sword_model_render_group,
            render_group_sphere.clone(),
        ];
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let tex_view = create_multisampled_framebuffer(&device, &config);
        let camera_controller = camera::CameraController::new(4.0, 0.2);

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
            depth_texture,
            render_groups,
            light_render_group,
            render_group_sphere,
            total_duration: Duration::from_secs(0),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.camera
                .projection
                .resize(new_size.width, new_size.height);
            self.surface.configure(&self.device, &self.config);
            self.tex_view = create_multisampled_framebuffer(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
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
                state: ElementState::Pressed,
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
        self.camera_controller
            .update_camera(&mut self.camera.view, dt);
        self.camera.update_camera(&self.queue);
        self.light_render_group
            .borrow_mut()
            .update_light(dt, &self.queue, self);
        self.total_duration += dt;
        let count = (3 + self.total_duration.as_secs() % 15) as usize;
        self.render_group_sphere.borrow_mut().entity.obj =
            create_sphere(10.0, count, count - 1, &self.device);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.frame_count += 1;
        // println!("frame count: {}", self.frame_count);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let refs: Vec<_> = self.render_groups.iter().map(|x| x.borrow()).collect();
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: if SAMPLE_COUNT == 1 {
                        &view
                    } else {
                        &self.tex_view
                    },
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

            refs.iter().for_each(|x| {
                x.render(&mut render_pass);
            });
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
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
    let mut window = WindowBuilder::new();

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
        log::warn!("Window size: {:?} {:?}", width, height);
        // window.set_inner_size(LogicalSize::new(width, height));
        use wasm_bindgen::JsCast;
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                dst.dyn_into::<web_sys::HtmlCanvasElement>()
                    .map_err(|_| ())
                    .ok()
            });
        use winit::platform::web::WindowBuilderExtWebSys;
        window = window
            .with_inner_size(LogicalSize::new(width, height))
            .with_canvas(canvas);
        // .expect("Couldn't append canvas to document body.");
    }

    let window = window.build(&event_loop).unwrap();
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
