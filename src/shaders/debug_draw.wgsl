struct SceneUniforms {
	perspective: mat4x4<f32>,
	view: mat4x4<f32>,
    inverse_perspective: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    camera_position: vec3<f32>,
    aspect_ratio: f32,
}

struct MeshUniforms {
	model: mat4x4<f32>,
	random_color: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<uniform> mesh: MeshUniforms;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
}

struct VertexOutput {
	@builtin(position) position_clip: vec4<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
	var out: VertexOutput;
	out.position_clip = scene.perspective * scene.view * mesh.model * vec4<f32>(in.position, 1.0);
	return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return vec4<f32>(mesh.random_color.rgb, 1.0);
}

