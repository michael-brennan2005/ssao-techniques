use std::{borrow::Cow, num::NonZeroU64};

pub use wgpu::{
    AddressMode, BufferUsages, CompareFunction, FilterMode, SamplerBindingType, ShaderStages,
    TextureFormat, TextureSampleType, TextureUsages, VertexBufferLayout,
};

// MARK: Descriptors
pub struct BufferDesc<'a> {
    pub label: Option<&'a str>,
    pub byte_size: u64,
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

pub struct BindGroupLayoutDesc<'a> {
    pub label: Option<&'a str>,
    pub visibility: ShaderStages,
    pub buffers: &'a [u64],
    pub textures: &'a [TextureSampleType],
    pub samplers: &'a [SamplerBindingType],
}

impl Default for BindGroupLayoutDesc<'_> {
    fn default() -> Self {
        BindGroupLayoutDesc {
            label: None,
            visibility: ShaderStages::all(),
            buffers: &[],
            textures: &[],
            samplers: &[],
        }
    }
}

pub struct BindGroupDesc<'a> {
    pub label: Option<&'a str>,
    pub visibility: ShaderStages,
    pub layout: Handle,
    pub buffers: &'a [Handle],
    pub textures: &'a [Handle],
    pub samplers: &'a [Handle],
}

impl Default for BindGroupDesc<'_> {
    fn default() -> Self {
        BindGroupDesc {
            label: None,
            layout: Handle(0),
            visibility: ShaderStages::all(),
            buffers: &[],
            textures: &[],
            samplers: &[],
        }
    }
}

#[derive(Clone, Copy)]
pub struct ShaderModuleDesc<'a> {
    pub path: &'a str,
    pub entry_func: &'a str,
}

#[derive(Clone, Copy)]
pub struct ShaderPipelineDesc<'a> {
    pub depth_test: Option<CompareFunction>,
    pub targets: &'a [TextureFormat],
    pub vertex_buffer_bindings: &'a [VertexBufferLayout<'a>],
}

#[derive(Clone, Copy)]
pub struct ShaderDesc<'a> {
    pub label: Option<&'a str>,
    pub vs: ShaderModuleDesc<'a>,
    pub ps: Option<ShaderModuleDesc<'a>>,
    pub bind_group_layouts: &'a [Handle],
    pub pipeline_state: ShaderPipelineDesc<'a>,
}

impl Default for ShaderDesc<'_> {
    fn default() -> Self {
        ShaderDesc {
            label: None,
            vs: ShaderModuleDesc {
                path: "",
                entry_func: "vs_main",
            },
            ps: None,
            bind_group_layouts: &[],
            pipeline_state: ShaderPipelineDesc {
                depth_test: None,
                targets: &[],
                vertex_buffer_bindings: &[],
            },
        }
    }
}

// MARK: Resources
pub struct Buffer {
    internal: wgpu::Buffer,
}

pub struct Texture {
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

pub struct BindGroupLayout {
    internal: wgpu::BindGroupLayout,
}

pub struct BindGroup {
    internal: wgpu::BindGroup,
}

pub struct Shader {
    internal: wgpu::RenderPipeline,
}

impl Shader {
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.internal
    }
}

// MARK: Resource manager
#[derive(Clone, Copy)]
pub struct Handle(usize);

pub struct ResourceManager {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_configuration: wgpu::SurfaceConfiguration,

    buffers: Vec<Buffer>,
    textures: Vec<Texture>,
    samplers: Vec<Sampler>,
    bind_group_layouts: Vec<BindGroupLayout>,
    bind_groups: Vec<BindGroup>,
    shaders: Vec<Shader>,
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
            bind_group_layouts: vec![],
            bind_groups: vec![],
            shaders: vec![],
        }
    }

    pub fn create_buffer(&mut self, desc: &BufferDesc) -> Handle {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label,
            size: desc.byte_size,
            usage: desc.usage,
            mapped_at_creation: false,
        });

        if let Some(data) = desc.initial_data {
            self.queue.write_buffer(&buffer, 0, data);
        }

        self.buffers.push(Buffer { internal: buffer });

        Handle(self.buffers.len() - 1)
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
        });

        Handle(self.textures.len() - 1)
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

        Handle(self.samplers.len() - 1)
    }

    pub fn create_bind_group_layout(&mut self, desc: BindGroupLayoutDesc) -> Handle {
        let mut i = 0;
        let mut entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];

        for entry in desc.buffers {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(*entry),
                },
                count: None,
            });

            i += 1;
        }

        for entry in desc.textures {
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

        for entry in desc.samplers {
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
                    label: desc.label,
                    entries: entries.as_slice(),
                });

        self.bind_group_layouts.push(BindGroupLayout {
            internal: bind_group_layout,
        });

        Handle(self.bind_group_layouts.len() - 1)
    }

    pub fn create_bind_group(&mut self, desc: BindGroupDesc) -> Handle {
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
            layout: &self.bind_group_layouts[desc.layout.0].internal,
            entries: entries.as_slice(),
        });

        self.bind_groups.push(BindGroup {
            internal: bind_group,
        });

        Handle(self.bind_group_layouts.len() - 1)
    }

    pub fn create_shader(&mut self, desc: &ShaderDesc) -> Handle {
        if desc.ps.is_some() && desc.ps.unwrap().path != desc.vs.path {
            panic!("only supporting ps and vs shaders from same file right now")
        }

        let source = std::fs::read_to_string(desc.vs.path).unwrap();

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(desc.vs.path),
                source: wgpu::ShaderSource::Wgsl(Cow::from(source.as_str())),
            });

        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = vec![];
        for entry in desc.bind_group_layouts {
            bind_group_layouts.push(&self.bind_group_layouts[entry.0].internal);
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

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(&self.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: bind_group_layouts.as_slice(),
                        push_constant_ranges: &[],
                    },
                )),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: desc.vs.entry_func,
                    buffers: desc.pipeline_state.vertex_buffer_bindings,
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
                        entry_point: desc.ps.unwrap().entry_func,
                        targets: &targets,
                    })
                } else {
                    None
                },
                multiview: None,
            });

        self.shaders.push(Shader { internal: pipeline });

        Handle(self.shaders.len() - 1)
    }

    pub fn get_texture(&self, handle: Handle) -> &Texture {
        &self.textures[handle.0]
    }

    pub fn get_shader(&self, handle: Handle) -> &Shader {
        &self.shaders[handle.0]
    }

    pub fn update_buffer(&self, handle: Handle, data: &[u8]) {
        self.queue
            .write_buffer(&self.buffers[handle.0].internal, 0, data);
    }
}
