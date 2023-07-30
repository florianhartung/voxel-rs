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

    out.color = mix(vec3(0.4941, 0.6627, 1.0), brightness * model.color, fog);
    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
