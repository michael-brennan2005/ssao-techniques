use std::path::Path;

use glam::{vec4, Mat4, Quat, Vec3, Vec4};
use gltf::buffer::Data;
use rand::Rng;
use wgpu::ShaderStages;

use crate::resource_manager::{
    BindGroupDesc, BindGroupLayoutDesc, BufferDesc, BufferUsages, Handle, ResourceManager,
};

macro_rules! bytemuck_impl {
    ($struct_name:ident) => {
        unsafe impl bytemuck::Pod for $struct_name {}
        unsafe impl bytemuck::Zeroable for $struct_name {}
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SceneUniformData {
    pub perspective: Mat4,
    pub view: Mat4,
    pub inverse_perspective: Mat4,
    pub inverse_view: Mat4,
    pub camera_position: Vec3,
    pub aspect_ratio: f32,
}
bytemuck_impl!(SceneUniformData);

impl Default for SceneUniformData {
    fn default() -> Self {
        Self {
            perspective: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            inverse_perspective: Mat4::IDENTITY,
            inverse_view: Mat4::IDENTITY,
            camera_position: Vec3::ONE,
            aspect_ratio: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VertexAttributes {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}
bytemuck_impl!(VertexAttributes);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshUniformData {
    pub model: Mat4,
    pub random_color: Vec4,
}
bytemuck_impl!(MeshUniformData);

pub struct Mesh {
    pub uniform_buffer: Handle,
    pub bind_group: Handle,
    pub vertex_buffer: Handle,
    pub index_buffer: Handle,
}

impl Mesh {
    pub fn new(
        rm: &mut ResourceManager,
        uniform_buffer: Handle,
        vertex_buffer: Handle,
        index_buffer: Handle,
    ) -> Self {
        let bind_group = rm.create_bind_group(&BindGroupDesc {
            label: None,
            visibility: ShaderStages::all(),
            layout: Mesh::bind_group_layout(&rm),
            buffers: &[uniform_buffer],
            textures: &[],
            samplers: &[],
        });

        Self {
            uniform_buffer,
            bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn bind_group_layout(rm: &ResourceManager) -> BindGroupLayoutDesc {
        BindGroupLayoutDesc {
            label: None,
            visibility: ShaderStages::all(),
            buffers: vec![std::mem::size_of::<MeshUniformData>()],
            textures: vec![],
            samplers: vec![],
        }
    }
}

pub struct Scene {
    pub scene_uniform: Handle,
    pub meshes: Vec<Mesh>,
}

impl Scene {
    fn walk_gltf(
        rm: &mut ResourceManager,
        node: &gltf::Node,
        original_transform: Mat4,
        buffers: &Vec<Data>,
    ) -> Vec<Mesh> {
        let (translation, rotation, scale) = node.transform().decomposed();

        let rotation_fixed = [rotation[0], rotation[1], rotation[2], rotation[3]];
        let translation_fixed = [translation[0], translation[1], translation[2]];
        // TODO: shouldn't this be backwards? test with some scenes
        let transform = original_transform
            * Mat4::from_scale_rotation_translation(
                scale.into(),
                Quat::from_array(rotation_fixed),
                translation_fixed.into(),
            );

        let mut meshes: Vec<Mesh> = Vec::new();

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| {
                    if buffer.index() < buffers.len() {
                        Some(buffers[buffer.index()].0.as_slice())
                    } else {
                        None
                    }
                });

                let indices = reader
                    .read_indices()
                    .expect("Couldn't read indices")
                    .into_u32()
                    .collect::<Vec<_>>();
                let positions = reader
                    .read_positions()
                    .expect("Couldn't read positions")
                    .map(|pos| [pos[0], pos[1], pos[2]]);
                let normals = reader
                    .read_normals()
                    .expect("Couldn't read normals")
                    .map(|pos| [pos[0], pos[1], pos[2]]);

                let mut vertices = positions
                    .zip(normals)
                    .map(|(position, normal)| VertexAttributes { position, normal })
                    .collect::<Vec<_>>();

                let uniform_buffer = rm.create_buffer(&BufferDesc {
                    label: None,
                    byte_size: std::mem::size_of::<MeshUniformData>(),
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    initial_data: Some(bytemuck::cast_slice(&[MeshUniformData {
                        model: transform,
                        random_color: vec4(
                            rand::thread_rng().gen_range(0.0..1.0),
                            rand::thread_rng().gen_range(0.0..1.0),
                            rand::thread_rng().gen_range(0.0..1.0),
                            1.0,
                        ),
                    }])),
                });

                let vertex_buffer = rm.create_buffer(&BufferDesc {
                    label: None,
                    byte_size: vertices.len() * std::mem::size_of::<VertexAttributes>(),
                    usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
                    initial_data: Some(bytemuck::cast_slice(vertices.as_slice())),
                });

                let index_buffer = rm.create_buffer(&BufferDesc {
                    label: None,
                    byte_size: indices.len() * std::mem::size_of::<u32>(),
                    usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
                    initial_data: Some(bytemuck::cast_slice(indices.as_slice())),
                });

                meshes.push(Mesh::new(rm, uniform_buffer, vertex_buffer, index_buffer));
            }
        }

        for child in node.children() {
            meshes.append(&mut Scene::walk_gltf(rm, &child, transform, buffers));
        }

        meshes
    }

    pub fn load_gltf(rm: &mut ResourceManager, path: &String) -> Self {
        let gltf = gltf::Gltf::open(path).expect("Gltf loading failed");
        let buffers = gltf::import_buffers(
            &gltf.document,
            Some(&Path::new(path).parent().unwrap_or_else(|| Path::new("./"))),
            None,
        )
        .expect("Buffer loading failed");
        let mut meshes: Vec<Mesh> = Vec::new();

        for node in gltf.nodes() {
            meshes.append(&mut Scene::walk_gltf(rm, &node, Mat4::IDENTITY, &buffers));
        }

        let scene_uniform = rm.create_buffer(&BufferDesc {
            label: Some("Scene uniform buffer"),
            byte_size: std::mem::size_of::<SceneUniformData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            initial_data: Some(bytemuck::cast_slice(&[SceneUniformData::default()])),
        });

        Self {
            scene_uniform,
            meshes,
        }
    }

    pub fn new(rm: &mut ResourceManager) -> Self {
        let scene_uniform = rm.create_buffer(&BufferDesc {
            label: Some("Scene uniform buffer"),
            byte_size: std::mem::size_of::<SceneUniformData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            initial_data: Some(bytemuck::cast_slice(&[SceneUniformData::default()])),
        });

        Self {
            scene_uniform,
            meshes: vec![],
        }
    }
}
