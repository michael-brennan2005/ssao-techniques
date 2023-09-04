use egui::{ClippedPrimitive, TexturesDelta};
use pollster::block_on;
use renderer::Renderer;
use resource_manager::ResourceManager;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{self, ControlFlow},
    window::WindowBuilder,
};

pub const WIDTH: u32 = 1600;
pub const HEIGHT: u32 = 900;
pub const BACKEND: wgpu::Backends = wgpu::Backends::DX12;

mod camera;
mod crytek_ssao;
mod renderer;
mod resource_manager;
mod scene;
mod texture_debug_view;

pub struct EguiRenderData {
    clipped_primitives: Vec<ClippedPrimitive>,
    textures_delta: TexturesDelta,
    screen_descriptor: ScreenDescriptor,
}

#[derive(Clone, Copy)]
pub struct ScreenDescriptor {
    size_in_pixels: [u32; 2],
    pixels_per_point: f32,
}

impl From<&egui_wgpu::renderer::ScreenDescriptor> for ScreenDescriptor {
    fn from(value: &egui_wgpu::renderer::ScreenDescriptor) -> Self {
        ScreenDescriptor {
            size_in_pixels: value.size_in_pixels,
            pixels_per_point: value.pixels_per_point,
        }
    }
}

impl Into<egui_wgpu::renderer::ScreenDescriptor> for ScreenDescriptor {
    fn into(self) -> egui_wgpu::renderer::ScreenDescriptor {
        egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: self.size_in_pixels,
            pixels_per_point: self.pixels_per_point,
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = event_loop::EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .with_title("SSAO techniques")
        .build(&event_loop)
        .unwrap();

    let mut egui_state = egui_winit::State::new(&event_loop);
    let egui_context = egui::Context::default();
    let egui_screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
        size_in_pixels: [WIDTH, HEIGHT],
        pixels_per_point: window.scale_factor() as f32,
    };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: BACKEND,
        dx12_shader_compiler: Default::default(),
    });

    let surface = unsafe { instance.create_surface(&window) }.unwrap();

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Device"),
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        },
        None,
    ))
    .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
        format: surface_format,
        width: WIDTH,
        height: HEIGHT,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let resource_manager = ResourceManager::new(device, queue, surface, config);
    let mut renderer = Renderer::new(resource_manager);

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent { window_id, event } if window_id == window.id() => {
            _ = egui_state.on_event(&egui_context, &event);
            renderer.input(&event);
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::ExitWithCode(0),
                _ => {}
            }
        }
        winit::event::Event::RedrawRequested(window_id) if window_id == window.id() => {
            let raw_input = egui_state.take_egui_input(&window);
            let ui_output = egui_context.run(raw_input, |ctx| {
                renderer.ui(ctx);
            });

            egui_state.handle_platform_output(&window, &egui_context, ui_output.platform_output);
            let clipped_primitives = egui_context.tessellate(ui_output.shapes);

            let egui_render_data = EguiRenderData {
                clipped_primitives,
                textures_delta: ui_output.textures_delta,
                screen_descriptor: ScreenDescriptor::from(&egui_screen_descriptor),
            };

            renderer.update(egui_render_data);
        }
        winit::event::Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}
