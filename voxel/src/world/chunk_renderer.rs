use std::collections::HashMap;

use wgpu::util::DeviceExt;
use wgpu::{include_wgsl, BufferUsages, PushConstantRange, ShaderStages};

use crate::rendering::texture::Texture;
use crate::rendering::{RenderCtx, Renderer};
use crate::world::chunk_data::ChunkData;
use crate::world::chunk_renderer::meshing::{ChunkMeshGenerator, NeighborChunks};
use crate::world::chunk_renderer::vertex::Vertex;
use crate::world::location::ChunkLocation;

pub mod meshing;
pub mod vertex;

pub struct ChunkRenderManager {
    renderers: HashMap<ChunkLocation, ChunkRenderer>,

    render_pipeline: wgpu::RenderPipeline,
}

impl ChunkRenderManager {
    pub fn new(ctx: &RenderCtx, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh render pipeline layout"),
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..12,
                }],
                bind_group_layouts: &[camera_bind_group_layout],
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

        Self {
            renderers: HashMap::new(),
            render_pipeline,
        }
    }

    pub fn generate_chunk_renderer(
        &mut self,
        chunk_data: &ChunkData,
        neighbor_chunks: NeighborChunks,
        ctx: &RenderCtx,
        chunk_location: ChunkLocation,
    ) {
        let quads = ChunkMeshGenerator::generate_culled_mesh(chunk_data, neighbor_chunks);

        let (vertices, mut indices) = ChunkMeshGenerator::generate_mesh_from_quads(quads);

        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunks vertex buffer"),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&vertices),
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunks index buffer"),
                usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&indices),
            });

        let renderer = ChunkRenderer {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        };

        self.renderers.insert(chunk_location, renderer);
    }
}

impl Renderer for ChunkRenderManager {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup, render_ctx: &RenderCtx) {
        for (position, renderer) in &self.renderers {
            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
            render_pass.set_index_buffer(renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.set_bind_group(0, camera_bind_group, &[]);

            // Push current chunk location
            let loc = [position.to_world_location_f32()];
            render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&loc));

            render_pass.draw_indexed(0..renderer.num_indices, 0, 0..1);
        }
    }
}

pub struct ChunkRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}
