use std::collections::HashMap;
use std::sync::Arc;

use vertex::Instance;
use wgpu::util::DeviceExt;
use wgpu::{BufferUsages, PipelineCompilationOptions, PushConstantRange, ShaderStages, VertexBufferLayout, include_wgsl};

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
                    buffers: &[Vertex::layout(), Instance::layout()],
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

#[derive(Debug)]
pub struct ChunkRenderer {
    pub instance_buffer: Option<wgpu::Buffer>,
    pub num_instances: u32,
}
