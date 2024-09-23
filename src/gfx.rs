use std::{fs::File, sync::Arc, time::{Duration, SystemTime}};
use pollster::FutureExt as _;

use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};

use wgpu::{Adapter, BindGroup, BindGroupLayout, Device, Instance, MemoryHints, PresentMode, Queue, Surface, SurfaceCapabilities, BindGroupLayoutDescriptor, Texture};

use crate::parser::Decoder;

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut window_state = StateApplication::new();
    let _ = event_loop.run_app(&mut window_state);

}

struct StateApplication<'a> {
    state: Option<State<'a>>,
}

impl<'a> StateApplication<'a> {
    pub fn new() -> Self {
        Self {
            state: None
        }
    }
}

impl<'a> ApplicationHandler for StateApplication<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        self.state = Some(State::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.state.as_ref().unwrap().window();

        if window.id() == window_id {
            match event {
                WindowEvent::CloseRequested => {
                    println!("close has been requested");
                    event_loop.exit();
                },
                WindowEvent::Resized(physical_size) => {
                    self.state.as_mut().unwrap().resize(physical_size);
                },
                WindowEvent::RedrawRequested => {
                    self.state.as_mut().unwrap().render().unwrap();
                },
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.state.as_ref().unwrap().window();
        window.request_redraw();
    }
}

struct State<'a> {
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
    decoder: Decoder<File>,
    texture_bind_group: BindGroup,
    texture: Texture,
    last_rendered: Option<SystemTime>,

    size: PhysicalSize<u32>,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
    frame_idx: usize,
}

impl<'a> State<'a> {
    pub fn new(window: Window) -> Self {
        let window_arc = Arc::new(window);
        let size = window_arc.inner_size();
        let instance = Self::create_gpu_instance();
        let surface = instance.create_surface(window_arc.clone()).unwrap();
        let adapter = Self::create_adapter(instance, &surface);
        let (device, queue) = Self::create_device(&adapter);
        let surface_caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(size, surface_caps);
        let file = File::open("./homeless-nah-id-win.gif").unwrap();

        let mut decoder = Decoder::new(file);
        decoder.parse().unwrap();

        let (texture_bind_group, texture_bind_group_layout, texture) = Self::create_texture_bind_group(&decoder, &device, &queue);
        let render_pipeline = Self::create_render_pipeline(&device, &config, &texture_bind_group_layout);

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            texture_bind_group,
            texture,
            render_pipeline,
            window: window_arc,
            decoder,
            frame_idx: 0,
            last_rendered: None
        }
    }

    fn create_texture_bind_group(decoder: &Decoder<File>, device: &Device, queue: &Queue) -> (BindGroup, BindGroupLayout, Texture) {
        let frame = decoder.frames().first().unwrap();

        let texture_size = wgpu::Extent3d {
            width: frame.width as u32,
            height: frame.height as u32,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            }
        );

        let palette = frame.palette().unwrap();
        let texture_buffer: Vec<u8> = frame
            .indicies()
            .iter()
            .flat_map(|index| {
                let color_idx = (*index as usize) * 3;

                let red = *palette.get(color_idx).unwrap();
                let green = *palette.get(color_idx + 1).unwrap();
                let blue = *palette.get(color_idx + 2).unwrap();

                vec![red, green, blue, 1]
            })
            .collect();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            &texture_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * frame.width as u32),
                rows_per_image: Some(frame.height as u32),
            },
            texture_size
        );

        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                }
            ],
            label: None
        });

        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_view)
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_sampler)
                    },
                ],
                label: None
            }
        );

        (diffuse_bind_group, texture_bind_group_layout, diffuse_texture)
    }

    fn create_surface_config(size: PhysicalSize<u32>, capabilities: SurfaceCapabilities) -> wgpu::SurfaceConfiguration {
        let surface_format = capabilities.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoNoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    fn create_device(adapter: &Adapter) -> (Device, Queue) {
        adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: MemoryHints::Performance,
                label: None,
            },
            None
        ).block_on().unwrap()
    }

    fn create_adapter(instance: Instance, surface: &Surface) -> Adapter {
        instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).block_on().unwrap()
    }

    fn create_gpu_instance() -> Instance {
        Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;

        self.config.width = new_size.width;
        self.config.height = new_size.height;

        self.surface.configure(&self.device, &self.config);

        println!("Resized to {:?} from state!", new_size);
    }

    pub fn write_next_texture(&mut self) {
        let frame = self.decoder.frames().get(self.frame_idx).unwrap();
        let should_render = match self.last_rendered {
            Some(time) => {
                time.elapsed().unwrap() >= Duration::from_millis(frame.delay_time.into())
            },
            None => {
                self.last_rendered = Some(SystemTime::now());
                true
            }
        };

        if !should_render {
            return
        }

        self.frame_idx += 1;
        if self.frame_idx == self.decoder.frames().len() {
            self.frame_idx = 0;
        }

        let texture_size = wgpu::Extent3d {
            width: frame.width as u32,
            height: frame.height as u32,
            depth_or_array_layers: 1,
        };

        let palette = frame.palette().unwrap();
        let texture_buffer: Vec<u8> = frame
            .indicies()
            .iter()
            .flat_map(|index| {
                let color_idx = (*index as usize) * 3;

                let red = *palette.get(color_idx).unwrap();
                let green = *palette.get(color_idx + 1).unwrap();
                let blue = *palette.get(color_idx + 2).unwrap();

                vec![red, green, blue, 1]
            })
            .collect();

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            &texture_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * frame.width as u32),
                rows_per_image: Some(frame.height as u32),
            },
            texture_size
        );

    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.write_next_texture();
        let output = self.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    }
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn create_render_pipeline(device: &Device, config: &wgpu::SurfaceConfiguration, bind_group_layout: &BindGroupLayout) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }
}
