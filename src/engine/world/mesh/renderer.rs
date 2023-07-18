use wgpu::util::DeviceExt;
use wgpu::{include_wgsl, vertex_attr_array, Face};

use crate::engine::rendering::texture::Texture;
use crate::engine::rendering::{HasBufferLayout, RenderCtx, Renderer};
use crate::engine::world::mesh::MeshVertex;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct RawMeshVertex {
    position: [f32; 3],
}

impl HasBufferLayout for RawMeshVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x3];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            attributes: &ATTRIBUTES,
            step_mode: wgpu::VertexStepMode::Vertex,
        }
    }
}

pub struct MeshRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl MeshRenderer {
    pub fn new(
        render_ctx: &RenderCtx,
        vertices: &[MeshVertex],
        indices: &[u32],
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let (vertex_buffer, index_buffer) = create_vert_ind_buffers(vertices, indices, render_ctx);

        let shader = render_ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            render_ctx
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Mesh render pipeline layout"),
                    push_constant_ranges: &[],
                    bind_group_layouts: &[camera_bind_group_layout],
                });

        let render_pipeline =
            render_ctx
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Default render pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        buffers: &[RawMeshVertex::layout()],
                        entry_point: "vs_main",
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        targets: &[Some(wgpu::ColorTargetState {
                            format: render_ctx.surface_config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        entry_point: "fs_main",
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: Some(Face::Back),
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

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            render_pipeline,
        }
    }

    pub fn update_data(
        &mut self,
        vertices: &[MeshVertex],
        indices: &[u32],
        render_ctx: &RenderCtx,
    ) {
        let (vertex_buffer, index_buffer) = create_vert_ind_buffers(vertices, indices, render_ctx);
        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;
        self.num_indices = indices.len() as u32;
    }
}

impl Renderer for MeshRenderer {
    fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.set_bind_group(0, camera_bind_group, &[]);

        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

fn create_vert_ind_buffers(
    vertices: &[MeshVertex],
    indices: &[u32],
    render_ctx: &RenderCtx,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let raw_vertices: Vec<RawMeshVertex> = vertices
        .iter()
        .map(|vertex| RawMeshVertex {
            position: vertex.position,
        })
        .collect();

    let vertex_buffer = render_ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(raw_vertices.as_slice()),
        });

    let index_buffer = render_ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh index buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(indices),
        });
    (vertex_buffer, index_buffer)
}
