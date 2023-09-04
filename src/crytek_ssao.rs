use glam::{vec3, Vec3};
use half::f16;
use rand::prelude::*;
use wgpu::{SamplerBindingType, ShaderStages, TextureFormat, TextureSampleType, TextureUsages};

use crate::{
    resource_manager::{
        BindGroupDesc, BindGroupLayoutDesc, Handle, ResourceManager, SamplerDesc, ShaderDesc,
        ShaderModuleDesc, ShaderPipelineDesc, TextureDesc,
    },
    scene::SceneUniformData,
};

pub struct CrytekSSAO {
    samples_texture: Handle,
    depth_buffer_sampler: Handle,
    ssao_bind_group: Handle,
    ssao_shader: Handle,
}

const NUM_SAMPLES: usize = 16;

impl CrytekSSAO {
    pub fn new(rm: &mut ResourceManager, depth_buffer: Handle) -> Self {
        let mut rng = rand::thread_rng();
        // generate samples
        let mut data: Vec<f16> = vec![];

        for i in 0..NUM_SAMPLES {
            let mut sample = vec3(rng.gen(), rng.gen(), rng.gen());
            sample = sample.normalize();

            data.push(f16::from_f32(sample.x));
            data.push(f16::from_f32(sample.y));
            data.push(f16::from_f32(sample.z));
            data.push(f16::from_f32(1.0));
        }

        let samples_texture = rm.create_texture(&TextureDesc {
            label: Some("Samples texture"),
            dimensions: (16, 1),
            mipmaps: None,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            initial_data: Some(bytemuck::cast_slice(data.as_slice())),
        });

        let depth_buffer_sampler = rm.create_sampler(SamplerDesc {
            label: Some("Depth buffer sampler"),
            address_mode: wgpu::AddressMode::ClampToEdge,
            mag_min_filter: wgpu::FilterMode::Linear,
            mipmaps: None,
            compare: None,
        });

        let ssao_bind_group = rm.create_bind_group(&BindGroupDesc {
            label: None,
            visibility: ShaderStages::FRAGMENT,
            layout: CrytekSSAO::bind_group_layout(),
            buffers: &[],
            textures: &[samples_texture, depth_buffer_sampler],
            samplers: &[depth_buffer_sampler],
        });

        let ssao_shader = rm.create_shader(ShaderDesc {
            label: Some(String::from("SSAO shader")),
            vs: ShaderModuleDesc {
                path: String::from("src/shaders/crytek_ssao.wgsl"),
                entry_func: String::from("vs_main"),
            },
            ps: Some(ShaderModuleDesc {
                path: String::from("src/shaders/crytek_ssao.wgsl"),
                entry_func: String::from("fs_main"),
            }),
            bind_group_layouts: vec![
                BindGroupLayoutDesc {
                    label: None,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    buffers: vec![std::mem::size_of::<SceneUniformData>()],
                    textures: vec![],
                    samplers: vec![],
                },
                CrytekSSAO::bind_group_layout(),
            ],
            pipeline_state: ShaderPipelineDesc {
                depth_test: None,
                targets: vec![TextureFormat::Bgra8UnormSrgb],
                vertex_buffer_bindings: vec![],
            },
        });

        Self {
            samples_texture,
            depth_buffer_sampler,
            ssao_bind_group,
            ssao_shader,
        }
    }

    pub fn bind_group_layout() -> BindGroupLayoutDesc {
        BindGroupLayoutDesc {
            label: None,
            visibility: ShaderStages::FRAGMENT,
            buffers: vec![],
            textures: vec![
                TextureSampleType::Float { filterable: true },
                TextureSampleType::Depth,
            ],
            samplers: vec![SamplerBindingType::Filtering],
        }
    }
}
