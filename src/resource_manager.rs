use std::{borrow::Cow, collections::HashMap, num::NonZeroU64};

use egui::Color32;
use pollster::block_on;
pub use wgpu::{
    AddressMode, BufferAddress, BufferSlice, BufferUsages, CompareFunction, FilterMode,
    SamplerBindingType, ShaderStages, TextureFormat, TextureSampleType, TextureUsages,
    VertexAttribute, VertexStepMode,
};

// MARK: Descriptors
pub struct BufferDesc<'a> {
    pub label: Option<&'a str>,
    pub byte_size: usize,
    pub usage: BufferUsages,
    pub initial_data: Option<&'a [u8]>,
}

impl Default for BufferDesc<'_> {
    fn default() -> Self {
        BufferDesc {
            label: None,
            byte_size: 0,
            usage: BufferUsages::all(),
            initial_data: None,
        }
    }
}

pub struct TextureDesc<'a> {
    pub label: Option<&'a str>,
    pub dimensions: (u32, u32),
    pub mipmaps: Option<u32>,
    pub format: TextureFormat,
    pub usage: TextureUsages,
    pub initial_data: Option<&'a [u8]>,
}

impl Default for TextureDesc<'_> {
    fn default() -> Self {
        TextureDesc {
            label: None,
            dimensions: (0, 0),
            mipmaps: None,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::all(),
            initial_data: None,
        }
    }
}

pub struct SamplerDesc<'a> {
    pub label: Option<&'a str>,
    pub address_mode: AddressMode,
    pub mag_min_filter: FilterMode,
    pub mipmaps: Option<u32>,
    pub compare: Option<CompareFunction>,
}

impl Default for SamplerDesc<'_> {
    fn default() -> Self {
        SamplerDesc {
            label: None,
            address_mode: AddressMode::Repeat,
            mag_min_filter: FilterMode::Linear,
            mipmaps: None,
            compare: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutDesc {
    pub label: Option<String>,
    pub visibility: ShaderStages,
    pub buffers: Vec<usize>,
    pub textures: Vec<TextureSampleType>,
    pub samplers: Vec<SamplerBindingType>,
}

impl Default for BindGroupLayoutDesc {
    fn default() -> Self {
        BindGroupLayoutDesc {
            label: None,
            visibility: ShaderStages::all(),
            buffers: vec![],
            textures: vec![],
            samplers: vec![],
        }
    }
}

pub struct BindGroupDesc<'a> {
    pub label: Option<&'a str>,
    pub visibility: ShaderStages,
    pub layout: BindGroupLayoutDesc,
    pub buffers: &'a [Handle],
    pub textures: &'a [Handle],
    pub samplers: &'a [Handle],
}

impl Default for BindGroupDesc<'_> {
    fn default() -> Self {
        BindGroupDesc {
            label: None,
            layout: BindGroupLayoutDesc {
                label: None,
                visibility: ShaderStages::all(),
                buffers: vec![],
                textures: vec![],
                samplers: vec![],
            },
            visibility: ShaderStages::all(),
            buffers: &[],
            textures: &[],
            samplers: &[],
        }
    }
}

#[derive(Clone)]
pub struct VertexBufferLayout {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Clone)]
pub struct ShaderModuleDesc {
    pub path: String,
    pub entry_func: String,
}

#[derive(Clone)]
pub struct ShaderPipelineDesc {
    pub depth_test: Option<CompareFunction>,
    pub targets: Vec<TextureFormat>,
    pub vertex_buffer_bindings: Vec<VertexBufferLayout>,
}

#[derive(Clone)]
pub struct ShaderDesc {
    pub label: Option<String>,
    pub vs: ShaderModuleDesc,
    pub ps: Option<ShaderModuleDesc>,
    pub bind_group_layouts: Vec<BindGroupLayoutDesc>,
    pub pipeline_state: ShaderPipelineDesc,
}

impl Default for ShaderDesc {
    fn default() -> Self {
        ShaderDesc {
            label: None,
            vs: ShaderModuleDesc {
                path: String::from(""),
                entry_func: String::from("vs_main"),
            },
            ps: None,
            bind_group_layouts: vec![],
            pipeline_state: ShaderPipelineDesc {
                depth_test: None,
                targets: vec![],
                vertex_buffer_bindings: vec![],
            },
        }
    }
}

// MARK: Resources
pub struct Buffer {
    internal: wgpu::Buffer,
}

impl Buffer {
    pub fn slice(&self) -> BufferSlice {
        self.internal.slice(..)
    }
}

pub struct Texture {
    pub depth: bool,
    internal: wgpu::Texture,
    view: wgpu::TextureView,
}
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

impl Texture {
    pub fn depth_stencil_attachment(&self) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        })
    }
}

pub struct Sampler {
    internal: wgpu::Sampler,
}

pub struct BindGroup {
    internal: wgpu::BindGroup,
}

pub struct Shader {
    desc: ShaderDesc,
    internal: wgpu::RenderPipeline,
}

impl Shader {
    fn new(rm: &mut ResourceManager, desc: ShaderDesc) -> Self {
        if desc.ps.is_some() && desc.ps.as_ref().unwrap().path != desc.vs.path {
            panic!("only supporting ps and vs shaders from same file right now")
        }

        let source = std::fs::read_to_string(desc.vs.path.clone()).unwrap();

        let shader = rm
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(desc.vs.path.clone().as_str()),
                source: wgpu::ShaderSource::Wgsl(Cow::from(source.as_str())),
            });

        let mut bind_group_layouts: Vec<wgpu::BindGroupLayout> = vec![];
        for entry in &desc.bind_group_layouts {
            bind_group_layouts.push(rm.get_bind_group_layout(entry));
        }

        let targets = desc
            .pipeline_state
            .targets
            .iter()
            .map(|x| {
                Some(wgpu::ColorTargetState {
                    format: *x,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect::<Vec<Option<wgpu::ColorTargetState>>>();

        let mut buffers: Vec<wgpu::VertexBufferLayout> = vec![];
        for buffer in &desc.pipeline_state.vertex_buffer_bindings {
            buffers.push(wgpu::VertexBufferLayout {
                array_stride: buffer.array_stride,
                step_mode: buffer.step_mode,
                attributes: &buffer.attributes,
            });
        }

        let pipeline = rm
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label.as_deref(),
                layout: Some(
                    &rm.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: bind_group_layouts
                                .iter()
                                .map(|x| x)
                                .collect::<Vec<&wgpu::BindGroupLayout>>()
                                .as_slice(),
                            push_constant_ranges: &[],
                        }),
                ),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: desc.vs.entry_func.as_str(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: if let Some(depth_test) = desc.pipeline_state.depth_test {
                    Some(wgpu::DepthStencilState {
                        format: TextureFormat::Depth32Float, // FIXME: move into variable/ texture-impl constant
                        depth_write_enabled: true,
                        depth_compare: depth_test,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    })
                } else {
                    None
                },
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: if desc.ps.is_some() {
                    Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: desc.ps.as_ref().unwrap().entry_func.as_str(),
                        targets: &targets,
                    })
                } else {
                    None
                },
                multiview: None,
            });

        Self {
            desc,
            internal: pipeline,
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.internal
    }
}

// MARK: Resource manager
#[derive(Clone, Copy)]
pub struct Handle(usize, HandleType);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum HandleType {
    BUFFER,
    TEXTURE,
    SAMPLER,
    BINDGROUP,
    SHADER,
}

pub struct ResourceManager {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_configuration: wgpu::SurfaceConfiguration,

    buffers: Vec<Buffer>,
    textures: Vec<Texture>,
    samplers: Vec<Sampler>,
    bind_groups: Vec<BindGroup>,
    shaders: Vec<Shader>,

    shader_compilation_error: String,
}

impl ResourceManager {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface,
        surface_configuration: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            device,
            queue,
            surface,
            surface_configuration,

            buffers: vec![],
            textures: vec![],
            samplers: vec![],
            bind_groups: vec![],
            shaders: vec![],

            shader_compilation_error: String::new(),
        }
    }

    pub fn create_buffer(&mut self, desc: &BufferDesc) -> Handle {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label,
            size: desc.byte_size as u64,
            usage: desc.usage,
            mapped_at_creation: false,
        });

        if let Some(data) = desc.initial_data {
            self.queue.write_buffer(&buffer, 0, data);
        }

        self.buffers.push(Buffer { internal: buffer });

        Handle(self.buffers.len() - 1, HandleType::BUFFER)
    }

    pub fn create_texture(&mut self, desc: &TextureDesc) -> Handle {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: desc.label,
            size: wgpu::Extent3d {
                width: desc.dimensions.0,
                height: desc.dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: desc.mipmaps.unwrap_or(0) + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: desc.usage,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());

        let bytes_per_pixel = match desc.format {
            TextureFormat::Rgba8UnormSrgb => 4,
            TextureFormat::Depth32Float => 4,
            TextureFormat::Rgba16Float => 8,
            _ => panic!("Unsupported format {:?}", desc.format),
        };

        if let Some(data) = desc.initial_data {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_pixel * desc.dimensions.0),
                    rows_per_image: Some(desc.dimensions.1),
                },
                wgpu::Extent3d {
                    width: desc.dimensions.0,
                    height: desc.dimensions.1,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.textures.push(Texture {
            internal: texture,
            view,
            depth: match desc.format {
                TextureFormat::Depth16Unorm
                | TextureFormat::Depth24Plus
                | TextureFormat::Depth24PlusStencil8
                | TextureFormat::Depth32Float
                | TextureFormat::Depth32FloatStencil8 => true,
                _ => false,
            },
        });

        Handle(self.textures.len() - 1, HandleType::TEXTURE)
    }

    pub fn create_sampler(&mut self, desc: SamplerDesc) -> Handle {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.address_mode,
            address_mode_v: desc.address_mode,
            address_mode_w: desc.address_mode,
            mag_filter: desc.mag_min_filter,
            min_filter: desc.mag_min_filter,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: if let Some(mipmaps) = desc.mipmaps {
                mipmaps as f32
            } else {
                0.0
            },
            compare: desc.compare,
            anisotropy_clamp: 1,
            border_color: None,
        });

        self.samplers.push(Sampler { internal: sampler });

        Handle(self.samplers.len() - 1, HandleType::SAMPLER)
    }

    pub fn create_bind_group(&mut self, desc: &BindGroupDesc) -> Handle {
        let mut i = 0;
        let mut entries: Vec<wgpu::BindGroupEntry> = vec![];

        for entry in desc.buffers {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: self.buffers[entry.0].internal.as_entire_binding(),
            });

            i += 1;
        }

        for entry in desc.textures {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: wgpu::BindingResource::TextureView(&self.textures[entry.0].view),
            });

            i += 1;
        }

        for entry in desc.samplers {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: wgpu::BindingResource::Sampler(&self.samplers[entry.0].internal),
            });

            i += 1;
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: desc.label,
            layout: &self.get_bind_group_layout(&desc.layout),
            entries: entries.as_slice(),
        });

        self.bind_groups.push(BindGroup {
            internal: bind_group,
        });

        Handle(self.bind_groups.len() - 1, HandleType::BINDGROUP)
    }

    pub fn create_shader(&mut self, desc: ShaderDesc) -> Handle {
        let shader = Shader::new(self, desc);

        self.shaders.push(shader);

        Handle(self.shaders.len() - 1, HandleType::SHADER)
    }

    pub fn get_buffer(&self, handle: Handle) -> &Buffer {
        if handle.1 != HandleType::BUFFER {
            panic!("Handle type is incorrect.");
        }
        &self.buffers[handle.0]
    }

    pub fn get_texture(&self, handle: Handle) -> &Texture {
        if handle.1 != HandleType::TEXTURE {
            panic!("Handle type is incorrect.");
        }
        &self.textures[handle.0]
    }

    pub fn get_shader(&self, handle: Handle) -> &Shader {
        if handle.1 != HandleType::SHADER {
            panic!("Handle type is incorrect.");
        }
        &self.shaders[handle.0]
    }

    fn get_bind_group_layout(&self, desc: &BindGroupLayoutDesc) -> wgpu::BindGroupLayout {
        let mut i = 0;
        let mut entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];

        for entry in &desc.buffers {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(*entry as u64),
                },
                count: None,
            });

            i += 1;
        }

        for entry in &desc.textures {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Texture {
                    sample_type: *entry,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });

            i += 1;
        }

        for entry in &desc.samplers {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Sampler(*entry),
                count: None,
            });

            i += 1;
        }

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: desc.label.as_deref(),
                    entries: entries.as_slice(),
                });

        bind_group_layout
    }

    pub fn get_bind_group(&self, handle: Handle) -> &wgpu::BindGroup {
        if handle.1 != HandleType::BINDGROUP {
            panic!("Expected handle type bindgroup, got {:?}", handle.1);
        }
        &self.bind_groups[handle.0].internal
    }

    pub fn update_buffer(&self, handle: Handle, data: &[u8]) {
        self.queue
            .write_buffer(&self.buffers[handle.0].internal, 0, data);
    }

    pub fn recompile(&mut self, handle: Handle) {
        let shader = &self.shaders[handle.0];

        let source = std::fs::read_to_string(shader.desc.vs.path.clone()).unwrap();

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.desc.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(Cow::from(source.as_str())),
            });
        let result = self.device.pop_error_scope();
        match block_on(result) {
            Some(err) => {
                self.shader_compilation_error = err.to_string();
            }
            None => {
                self.shader_compilation_error = String::new();
                self.shaders[handle.0] = Shader::new(self, shader.desc.clone());
            }
        }
    }

    pub fn egui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Buffers created: {}", self.buffers.len()));
        ui.label(format!("Textures created: {}", self.textures.len()));
        ui.label(format!("Samplers created: {}", self.samplers.len()));
        ui.label(format!("BindGroups created: {}", self.bind_groups.len()));
        ui.label(format!("Shaders created: {}", self.shaders.len()));

        ui.label(egui::RichText::new("Shaders").strong());
        egui::Grid::new("shaders").show(ui, |ui| {
            let paths: Vec<String> = self
                .shaders
                .iter()
                .map(|x| x.desc.vs.path.clone())
                .collect();

            for (i, path) in paths.iter().enumerate() {
                ui.label(path);
                if ui.button("Reload").clicked() {
                    self.recompile(Handle(i, HandleType::SHADER));
                }
                ui.end_row();
            }
        });

        ui.label(egui::RichText::new(&self.shader_compilation_error).color(Color32::RED));
    }
}
