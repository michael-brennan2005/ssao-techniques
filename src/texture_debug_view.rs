use wgpu::{
    vertex_attr_array, CommandEncoder, CompareFunction, ShaderStages, TextureFormat,
    TextureSampleType, TextureView,
};

use crate::{
    resource_manager::{
        BindGroupDesc, BindGroupLayoutDesc, Handle, ResourceManager, ShaderDesc, ShaderModuleDesc,
        ShaderPipelineDesc, VertexBufferLayout,
    },
    scene::{Mesh, SceneUniformData, VertexAttributes},
};

pub struct TextureDebugView {
    shader: Handle,
    bind_group: Handle,
}

impl TextureDebugView {
    pub fn bind_group_layout(depth: bool) -> BindGroupLayoutDesc {
        if depth {
            BindGroupLayoutDesc {
                label: None,
                visibility: ShaderStages::FRAGMENT,
                buffers: vec![],
                textures: vec![TextureSampleType::Depth],
                samplers: vec![],
            }
        } else {
            BindGroupLayoutDesc {
                label: None,
                visibility: ShaderStages::FRAGMENT,
                buffers: vec![],
                textures: vec![TextureSampleType::Float { filterable: true }],
                samplers: vec![],
            }
        }
    }

    pub fn new(rm: &mut ResourceManager, texture: Handle) -> Self {
        if rm.get_texture(texture).depth {
            println!("path 1");
            let shader = rm.create_shader(ShaderDesc {
                label: None,
                vs: ShaderModuleDesc {
                    path: String::from("src/shaders/texture_debug_depth.wgsl"),
                    entry_func: String::from("vs_main"),
                },
                ps: Some(ShaderModuleDesc {
                    path: String::from("src/shaders/texture_debug_depth.wgsl"),
                    entry_func: String::from("fs_main"),
                }),
                bind_group_layouts: vec![TextureDebugView::bind_group_layout(true)],
                pipeline_state: ShaderPipelineDesc {
                    depth_test: None,
                    targets: vec![TextureFormat::Bgra8UnormSrgb],
                    vertex_buffer_bindings: vec![],
                },
            });

            let bind_group = rm.create_bind_group(&BindGroupDesc {
                label: None,
                visibility: ShaderStages::FRAGMENT,
                layout: TextureDebugView::bind_group_layout(true),
                buffers: &[],
                textures: &[texture],
                samplers: &[],
            });
            Self { shader, bind_group }
        } else {
            println!("path 2");
            let shader = rm.create_shader(ShaderDesc {
                label: None,
                vs: ShaderModuleDesc {
                    path: String::from("src/shaders/texture_debug.wgsl"),
                    entry_func: String::from("vs_main"),
                },
                ps: Some(ShaderModuleDesc {
                    path: String::from("src/shaders/texture_debug.wgsl"),
                    entry_func: String::from("fs_main"),
                }),
                bind_group_layouts: vec![TextureDebugView::bind_group_layout(false)],
                pipeline_state: ShaderPipelineDesc {
                    depth_test: None,
                    targets: vec![TextureFormat::Bgra8UnormSrgb],
                    vertex_buffer_bindings: vec![],
                },
            });

            let bind_group = rm.create_bind_group(&BindGroupDesc {
                label: None,
                visibility: ShaderStages::FRAGMENT,
                layout: TextureDebugView::bind_group_layout(false),
                buffers: &[],
                textures: &[texture],
                samplers: &[],
            });
            Self { shader, bind_group }
        }
    }

    pub fn pass(&self, rm: &ResourceManager, encoder: &mut CommandEncoder, view: &TextureView) {
        {
            let mut debug_view = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Debug texture view"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            debug_view.set_pipeline(rm.get_shader(self.shader).pipeline());
            debug_view.set_bind_group(0, rm.get_bind_group(self.bind_group), &[]);
            debug_view.draw(0..6, 0..1);
        }
    }
}
