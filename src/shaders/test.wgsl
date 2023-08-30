@vertex
fn vs_main(@builtin(vertex_index) in: u32) -> @builtin(position) vec4<f32> {
	if (in == 0u) {
		return vec4<f32>(0.0, 0.0, 0.0, 1.0);
	} else if (in == 1u) {
		return vec4<f32>(0.0, 0.5, 0.0, 1.0);
	} else {
		return vec4<f32>(0.5, 0.0, 0.0, 1.0);
	}
}

@fragment
fn fs_main(@builtin(position) x: vec4<f32>) -> @location(0) vec4<f32> {
	return vec4<f32>(1.0,1.0,0.0,1.0);
	
}
