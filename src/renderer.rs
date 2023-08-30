use winit::event::WindowEvent;

use crate::{
    camera::{Camera, CameraController, FlyCamera},
    resource_manager::{
        CompareFunction, Handle, ResourceManager, ShaderDesc, ShaderModuleDesc, ShaderPipelineDesc,
        TextureDesc, TextureFormat, TextureUsages, DEPTH_FORMAT,
    },
    scene::Scene,
    EguiRenderData,
};

pub struct Renderer {
    rm: ResourceManager,
    egui: egui_wgpu::Renderer,
    scene: Scene,

    camera: Camera,
    camera_controller: Box<dyn CameraController>,

    depth_buffer: Handle,
    shader: Handle,
}

impl Renderer {
    pub fn new(mut rm: ResourceManager) -> Self {
        let scene = Scene::new(&mut rm);

        let camera = Camera::default();
        let fly_camera = Box::new(FlyCamera::new());

        let depth_buffer = rm.create_texture(&TextureDesc {
            label: Some("Depth buffer"),
            dimensions: (
                rm.surface_configuration.width,
                rm.surface_configuration.height,
            ),
            mipmaps: None,
            format: DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            initial_data: None,
        });

        let shader = rm.create_shader(ShaderDesc {
            label: None,
            vs: ShaderModuleDesc {
                path: String::from("src/shaders/test.wgsl"),
                entry_func: String::from("vs_main"),
            },
            ps: Some(ShaderModuleDesc {
                path: String::from("src/shaders/test.wgsl"),
                entry_func: String::from("fs_main"),
            }),
            bind_group_layouts: vec![],
            pipeline_state: ShaderPipelineDesc {
                depth_test: Some(CompareFunction::Less),
                targets: vec![TextureFormat::Bgra8UnormSrgb],
                vertex_buffer_bindings: vec![],
            },
        });

        let egui = egui_wgpu::renderer::Renderer::new(
            &rm.device,
            rm.surface_configuration.format,
            None,
            1,
        );

        Self {
            scene,
            rm,
            depth_buffer,
            shader,
            egui,
            camera,
            camera_controller: fly_camera,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Renderer").show(ctx, |ui| {
            egui::CollapsingHeader::new("Resources").show(ui, |ui| {
                self.rm.egui(ui);
            });

            egui::CollapsingHeader::new("Loader").show(ui, |ui| {
                if ui.button("Load glTF").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("glTF", &["gltf", "glb"])
                        .pick_file()
                    {
                        self.scene =
                            Scene::load_gltf(&mut self.rm, &String::from(path.to_str().unwrap()));
                    }
                }
            });

            self.camera_controller.ui(&mut self.camera, ui);
        });
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera_controller.input(event);
    }

    pub fn update(&mut self, egui_render_data: EguiRenderData) {
        self.camera_controller.update(&mut self.camera);
        self.rm.update_buffer(
            self.scene.scene_uniform,
            bytemuck::cast_slice(&[self.camera.build_uniforms()]),
        );

        let output = self.rm.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .rm
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut test_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: self
                    .rm
                    .get_texture(self.depth_buffer)
                    .depth_stencil_attachment(),
            });

            test_pass.set_pipeline(self.rm.get_shader(self.shader).pipeline());
            test_pass.draw(0..3, 0..1);
        }

        self.render_egui(&view, &mut encoder, egui_render_data);
        self.rm.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn render_egui(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        egui_render_data: EguiRenderData,
    ) {
        for delta in &egui_render_data.textures_delta.set {
            self.egui
                .update_texture(&self.rm.device, &self.rm.queue, delta.0, &delta.1);
        }

        self.egui.update_buffers(
            &self.rm.device,
            &self.rm.queue,
            encoder,
            &egui_render_data.clipped_primitives,
            &egui_render_data.screen_descriptor.into(),
        );

        {
            let mut egui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.egui.render(
                &mut egui_pass,
                &egui_render_data.clipped_primitives,
                &egui_render_data.screen_descriptor.into(),
            );
        }

        for delta in &egui_render_data.textures_delta.free {
            self.egui.free_texture(delta);
        }
    }
}
