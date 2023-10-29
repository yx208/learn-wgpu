#![allow(dead_code)]

use winit::{
    event::*,
    event_loop::{ ControlFlow, EventLoop },
    window::{ Window, WindowBuilder }
};

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: &Window) -> Self {

        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window).unwrap() };
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .filter(|adapter| adapter.is_surface_supported(&surface))
            .next()
            .unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None
            },
            None
        ).await.unwrap();

        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[]
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })]
            }),
            // 图元（primitive）, 描述了将如何解释顶点来转换为三角形
            primitive: wgpu::PrimitiveState {
                // 每三个顶点组成一个三角形
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // 告诉 wgpu 如何确定三角形的朝向
                front_face: wgpu::FrontFace::Ccw,
                // 告诉 wgpu 如何做三角形剔除
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false
            },
            // 多重采样
            multisample: wgpu::MultisampleState {
                // 确定管线将使用多少个采样
                count: 1,
                // 哪些采样应处于活动状态。目前我们使用全部采样
                mask: !0,
                // 与抗锯齿有关
                alpha_to_coverage_enabled: false
            },
            depth_stencil: None,
            // 表示渲染附件可以有多少数组层，不会渲染到数组纹理
            multiview: None,
        });

        Self {
            size,
            surface,
            device,
            queue,
            config,
            render_pipeline
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {

    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder")
            }
        );

        // 渲染通道
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[
                // 这就是片元着色器中 @location(0) 标记指向的颜色附件
                Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // 告诉 wgpu 如何处理存储在前一帧的颜色
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0
                        }),
                        // 是否要将渲染的结果存储到纹理视图后面的纹理上
                        store: wgpu::StoreOp::Store
                    }
                })
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        // 告诉 wgpu 用 3 个顶点和 1 个实例
        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    run().await;
}

pub async fn run() {

    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // 除非我们手动请求，RedrawRequested 将只会触发一次。
                window.request_redraw();
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // 当展示平面的上下文丢失，就需重新配置
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // 系统内存不足时，程序应该退出。
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::ExitWithCode(0),
                    // 所有其他错误（过期、超时等）应在下一帧解决
                    Err(e) => eprintln!("{:?}", e)
                }
            }
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            input: KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => {
                            *control_flow = ControlFlow::ExitWithCode(0);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });

}
