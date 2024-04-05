struct CameraUniform {
	position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

//struct ModelUniform {
//	pos: vec2<f32>,
//}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage,read_write> chunk_pos_ssbo: array<vec3<i32>>;

struct VertexInput {
	@location(0) position_x_y_z_color_r: u32,
	@location(1) color_g_b_normal_ao: u32,
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
	var model_ao: f32 = parse_model_ao(model.color_g_b_normal_ao);

	var vertex_position = model_position;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4((vertex_position), 1.0);

    var brightness: f32;
    brightness = 0.2 + 0.8*dot(model_normal, vec3(1.0, 0.5, 0.7));

	var fog_dist: f32;
	fog_dist = distance(camera.position.xyz, vertex_position);
	var fog: f32 = 1.0 - max(0.0, min(1.0, pow(fog_dist / (32.0*32.0), 16.0)));

	var ambient_occlusion = model_ao / 3.0; // shadow 0.0 <-> 1.0 light

	var color = (brightness - 0.2 * (1.0 - ambient_occlusion)) * model_color;

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

fn parse_model_ao(model2: u32) -> f32 {
	return f32((model2 & 0x00001800u) >> 11u);
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
