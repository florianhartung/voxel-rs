use std::collections::HashMap;
use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu::{BufferUsages, PipelineCompilationOptions, PushConstantRange, ShaderStages, include_wgsl};

use crate::renderer::texture::Texture;
use crate::renderer::{RenderCtx, Renderer};
use crate::world::chunk_data::ChunkData;
use crate::world::chunk_renderer::meshing::{ChunkMeshGenerator, NeighborChunks};
use crate::world::chunk_renderer::vertex::Vertex;
use crate::world::location::ChunkLocation;

pub mod meshing;
pub mod vertex;

pub struct ChunkRenderPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
}

impl ChunkRenderPipeline {
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
                    entry_point: Some("vs_main"),
                    compilation_options: PipelineCompilationOptions::default(),
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
                    entry_point: Some("fs_main"),
                    compilation_options: PipelineCompilationOptions::default(),
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
                cache: None,
            });

        Self { render_pipeline }
    }
}

pub fn generate_chunk_renderer(chunk_data: &ChunkData, neighbor_chunks: NeighborChunks, ctx: &RenderCtx) -> Arc<ChunkRenderer> {
    let quads = ChunkMeshGenerator::generate_culled_mesh(&*chunk_data, neighbor_chunks);

    let (vertices, indices) = ChunkMeshGenerator::generate_mesh_from_quads(quads);

    let vertex_buffer = (vertices.len() > 0).then(|| {
        ctx.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunks vertex buffer"),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&vertices),
            })
    });

    let index_buffer = (indices.len() > 0).then(|| {
        ctx.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunks index buffer"),
                usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&indices),
            })
    });

    Arc::new(ChunkRenderer {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
    })
}

#[derive(Debug)]
pub struct ChunkRenderer {
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_indices: u32,
}
