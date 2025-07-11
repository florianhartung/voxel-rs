struct CameraUniform {
	position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

//struct ModelUniform {
//	pos: vec2<f32>,
//}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<push_constant> position_offset: vec3<f32>;

struct VertexInput {
	// per-vertex index inside the quad: 0,1,2,3 
	@location(0) n: u32,

	// per-instance
	@location(1) position_x_y_z_color_r: u32,
	@location(2) color_g_b_normal_ao: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
	var model_position: vec3<f32> = parse_model_position(model.position_x_y_z_color_r);
	var model_color: vec3<f32> = parse_model_color(model.position_x_y_z_color_r, model.color_g_b_normal_ao);
	var model_normal: vec3<f32> = parse_model_normal(model.color_g_b_normal_ao);
	var model_ao: vec4<u32> = parse_model_ao(model.color_g_b_normal_ao);
	var model_reversed_orientation: bool = parse_reversed_orientation(model.color_g_b_normal_ao);

	var normal_idx: u32 = (model.color_g_b_normal_ao & 0x0000E000u) >> 13u;
	var is_backside: bool;

	switch normal_idx {
		case 0, 1, 2: {
			is_backside = false;
		}
		case 3, 4, 5, default: {
			is_backside = true;
		}
	}

	var vertex_pos = vec3(0.0);
	var selected_ao: u32 = 3;

	var model_n = model.n;
	if model_reversed_orientation {
		switch model_n {
			case 0: {
				model_n = 2;
			}
			case 1: {
				model_n = 0;
			}
			case 2: {
				model_n = 3;
			}
			case 3, default: {
				model_n = 1;
			}
		}
	}
	switch normal_idx {
		case 0, 3: {
			if !is_backside {
				if model_n == 1 {
					model_n = 2;
				} else if model_n == 2 {
					model_n = 1;
				}
				vertex_pos += vec3(0.0, 0.0, 1.0);
			}
			switch model_n {
				case 0: {
					vertex_pos += vec3(0.0, 0.0, 0.0);
					selected_ao = model_ao.x;
				}
				case 1: {
					vertex_pos += vec3(1.0, 0.0, 0.0);
					selected_ao = model_ao.z;
				}
				case 2: {
					vertex_pos += vec3(0.0, 1.0, 0.0);
					selected_ao = model_ao.y;
				}
				case 3, default: {
					vertex_pos += vec3(1.0, 1.0, 0.0);
					selected_ao = model_ao.w;
				}
			}
		}
		case 1, 4: {
			if !is_backside {
				if model_n == 1 {
					model_n = 2;
				} else if model_n == 2 {
					model_n = 1;
				}
				vertex_pos += vec3(0.0, 1.0, 0.0);
			}
			switch model_n {
				case 0: {
					vertex_pos += vec3(0.0, 0.0, 0.0);
					selected_ao = model_ao.x;
				}
				case 1: {
					vertex_pos += vec3(0.0, 0.0, 1.0);
					selected_ao = model_ao.z;
				}
				case 2: {
					vertex_pos += vec3(1.0, 0.0, 0.0);
					selected_ao = model_ao.y;
				}
				case 3, default: {
					vertex_pos += vec3(1.0, 0.0, 1.0);
					selected_ao = model_ao.w;
				}
			}
		}
		case 2, 5, default: {
			if !is_backside {
				if model_n == 1 {
					model_n = 2;
				} else if model_n == 2 {
					model_n = 1;
				}

				vertex_pos += vec3(1.0, 0.0, 0.0);
			}

			switch model_n {
				case 0: {
					vertex_pos += vec3(0.0, 0.0, 0.0);
					selected_ao = model_ao.x;
				}
				case 1: {
					vertex_pos += vec3(0.0, 1.0, 0.0);
					selected_ao = model_ao.z;
				}
				case 2: {
					vertex_pos += vec3(0.0, 0.0, 1.0);
					selected_ao = model_ao.y;
				}
				case 3, default: {
					vertex_pos += vec3(0.0, 1.0, 1.0);
					selected_ao = model_ao.w;
				}
			}
		}
	}



	var vertex_position = vertex_pos + model_position + position_offset;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4((vertex_position), 1.0);

    var brightness: f32;
    brightness = 0.2 + 0.8*dot(abs(model_normal), vec3(1.0, 0.5, 0.7));

	var fog_dist: f32;
	fog_dist = distance(camera.position.xyz, vertex_position);
	var fog: f32 = 1.0 - max(0.0, min(1.0, pow(fog_dist / (32.0*32.0*10.0), 16.0)));

	var ambient_occlusion = f32(selected_ao) / 3.0; // shadow 0.0 <-> 1.0 light

	var color = (brightness - 0.3 * (1.0 - ambient_occlusion)) * model_color;

    out.color = mix(vec3(0.4941, 0.6627, 1.0), color, fog);

    return out;
}

fn parse_model_position(model1: u32) -> vec3<f32> {
	return vec3(
		f32((model1 & 0xFF000000u) >> 24u),
		f32((model1 & 0x00FF0000u) >> 16u),
		f32((model1 & 0x0000FF00u) >> 8u),
	);
}
fn parse_model_color(model1: u32, model2: u32) -> vec3<f32> {
	return vec3(
		f32(model1 & 0x000000FFu) / 255.0,
		f32((model2 & 0xFF000000u) >> 24u) / 255.0,
		f32((model2 & 0x00FF0000u) >> 16u) / 255.0,
	);
}

fn parse_model_normal(model2: u32) -> vec3<f32> {
	var NORMAL_LOOKUP = array<vec3<f32>, 6>(
		vec3(0.0, 0.0, 1.0),
		vec3(0.0, 1.0, 0.0),
		vec3(1.0, 0.0, 0.0),
		vec3(0.0, 0.0, -1.0),
		vec3(0.0, -1.0, 0.0),
		vec3(-1.0, 0.0, 0.0));

	var normal_idx: u32 = (model2 & 0x0000E000u) >> 13u;

	return NORMAL_LOOKUP[normal_idx];
}

fn parse_model_ao(model2: u32) -> vec4<u32> {
	var ao1 = ((model2 >> 11) & 3);
	var ao2 = ((model2 >> 9) & 3);
	var ao3 = ((model2 >> 7) & 3);
	var ao4 = ((model2 >> 5) & 3);
	return vec4<u32>(ao1, ao2, ao3, ao4);
}

fn parse_reversed_orientation(model2: u32) -> bool {
	return ((model2 >> 4) & 1) > 0;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}


// --- AO Coloring ---
//	var ao_color = vec3(0.0, 0.0, 0.0);
//	var eps = 0.01;
//	if (abs(ambient_occlusion - 0.0) < eps) {
//		ao_color.x = 1.0;
//	} else if (abs(ambient_occlusion - 0.3333) < eps) {
//		ao_color.y = 1.0;
//	} else if (abs(ambient_occlusion - 0.6666) < eps) {
//		ao_color.z = 1.0;
//	} else if (abs(ambient_occlusion - 1.0) < eps) {
//		ao_color.x = 1.0;
//		ao_color.y = 1.0;
//		ao_color.z = 1.0;
//	} else {
//		ao_color.y = 1.0;
//		ao_color.z = 1.0;
//	}
