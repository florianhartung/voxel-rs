use std::fmt::{Debug, Formatter};
use std::ops::Deref;

use bytemuck::{Pod, Zeroable};
use cgmath::num_traits::ToPrimitive;
use cgmath::Vector3;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{include_wgsl, vertex_attr_array, ShaderStages};

use crate::rendering::texture::Texture;
use crate::rendering::{RenderCtx, Renderer};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    renderer: MeshRenderer,
}

impl Debug for Mesh {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mesh{{renderer: {:?}}}", self.renderer)
    }
}

impl Mesh {
    pub fn new(
        render_ctx: impl Deref<Target = RenderCtx>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        position: Vector3<f32>,
    ) -> Self {
        let mesh_render = MeshRenderer::new(render_ctx, camera_bind_group_layout, &vertices, &indices, position);

        Self {
            vertices,
            indices,
            renderer: mesh_render,
        }
    }

    pub fn update(&mut self, new_vertices: Vec<Vertex>, new_indices: Vec<u32>) {
        self.vertices = new_vertices;
        self.indices = new_indices;

        self.renderer
            .update(&self.vertices, &self.indices);
    }

    pub fn get_renderer(&self) -> &MeshRenderer {
        &self.renderer
    }
}

/// Layout:
/// 0: u32
///   - x: u8
///   - y: u8
///   - z: u8
///   - r: u8
/// 1: u32
///   - g: u8
///   - b: u8
///   - normal: 3 bits:  0, 1, 2, 3, 4, 5 => (0, 0, 1), (0, 1, 0), (1, 0, 0), (0, 0, -1), (0, -1, 0), (-1, 0, 0)
///   - ao: 2 bits
///   - _unused: 11 bits
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position_x_y_z_color_r: u32,
    color_g_b_normal_ao: u32,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>, direction: Vector3<f32>, ambient_occlusion: f32) -> Self {
        let x: u8 = position.x.to_u8().unwrap();
        let y: u8 = position.y.to_u8().unwrap();
        let z: u8 = position.z.to_u8().unwrap();
        let r: u8 = (255.0 * color.x).to_u8().unwrap();
        let g: u8 = (255.0 * color.y).to_u8().unwrap();
        let b: u8 = (255.0 * color.z).to_u8().unwrap();

        let normal: u32 = match &direction[..] {
            &[0.0, 0.0, 1.0] => 0,
            &[0.0, 1.0, 0.0] => 1,
            &[1.0, 0.0, 0.0] => 2,
            &[0.0, 0.0, -1.0] => 3,
            &[0.0, -1.0, 0.0] => 4,
            &[-1.0, 0.0, 0.0] => 5,
            _ => panic!("invalid direction"),
        };

        let ao: u32 = ambient_occlusion.to_u32().unwrap();
        assert!((0..=3).contains(&ao));

        Self {
            position_x_y_z_color_r: u32::from_be_bytes([x, y, z, r]),
            color_g_b_normal_ao: u32::from_be_bytes([g, b, 0, 0]) | normal << 13 | ao << 11,
        }
    }

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] = vertex_attr_array![0 => Uint32, 1 => Uint32];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            attributes: &ATTRIBUTES,
            step_mode: wgpu::VertexStepMode::Vertex,
        }
    }
}

#[derive(Debug)]
pub struct MeshRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    render_pipeline: wgpu::RenderPipeline,

    /// The position offset of this mesh
    position_buffer: wgpu::Buffer,
    position_bind_group: wgpu::BindGroup,
    position_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub fn new(
        ctx: impl Deref<Target = RenderCtx>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        vertices: &Vec<Vertex>,
        indices: &Vec<u32>,
        position: Vector3<f32>,
    ) -> Self {
        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh vertex buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(vertices.as_slice()),
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh index buffer"),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(indices.as_slice()),
            });

        let buffer_data = [position.x, position.y, position.z, 0.0]; // 0.0 as padding for alignment
        let position_buffer = ctx
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some("Buffer for position offset of mesh"),
                contents: bytemuck::cast_slice(&buffer_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let position_bind_group_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind group layout for position offset of mesh"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: ShaderStages::VERTEX,
                    count: None,
                }],
            });

        let position_bind_group = ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind group for position offset of mesh"),
                layout: &position_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: position_buffer.as_entire_binding(),
                }],
            });

        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh render pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[camera_bind_group_layout, &position_bind_group_layout],
            });

        let render_pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default render pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    buffers: &[Vertex::layout()],
                    entry_point: "vs_main",
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx
                            .surface_config
                            .try_lock()
                            .expect("i also hope this isn't locked")
                            .format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    entry_point: "fs_main",
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Back),
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: Default::default(),
                multiview: None,
            });
        drop(ctx);

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            render_pipeline,
            position_buffer,
            position_bind_group_layout,
            position_bind_group,
        }
    }

    pub fn update(&mut self, _new_vertices: &[Vertex], _new_indices: &[u32]) {
        todo!("Update buffers")
    }
}

impl Renderer for MeshRenderer {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup) {
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.position_bind_group, &[]);

        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
