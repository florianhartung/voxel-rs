struct CameraUniform {
	position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

//struct ModelUniform {
//	pos: vec2<f32>,
//}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) ambient_occlusion: f32,
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
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4((model.position), 1.0) ;

    var brightness: f32;
    brightness = dot(model.normal, vec3(1.0, 0.5, 0.7));

	var fog_dist: f32;
	fog_dist = distance(camera.position.xyz, model.position);
	var fog: f32 = 1.0 - max(0.0, min(1.0, pow(fog_dist / (24.0*32.0), 2.0)));

	var ambient_occlusion = model.ambient_occlusion / 3.0;

	var color = (0.7*brightness + 0.3*ambient_occlusion) * model.color;

    out.color = mix(vec3(0.4941, 0.6627, 1.0), color, fog);
    return out;
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
