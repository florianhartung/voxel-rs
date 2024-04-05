use std::collections::HashMap;
use std::mem::size_of;

use cgmath::Vector3;
use log::info;
use wgpu::util::{DeviceExt, DrawIndexedIndirect};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferBindingType, BufferDescriptor, BufferUsages, ShaderStages,
};

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

    vertex_buffer: Buffer,
    num_vertices: usize,

    index_buffer: Buffer,
    num_indices: usize,

    // SSBO stores position per multi draw
    // position_ssbo: Vec<ChunkLocation>,
    // position_ssbo_buffer: Buffer,
    // position_bind_group: BindGroup,
    indirect_buffer: Buffer,
    // or num_chunks
    num_indirect: usize,
    chunk_loc_ssbo: Buffer,
    chunk_loc_ssbo_bind_group: BindGroup,
}

const MAX_VERTICES: usize = 10000000;
const MAX_INDICES: usize = 3000000;
const MAX_CHUNKS: usize = 260000;

impl ChunkRenderManager {
    pub fn new(ctx: &RenderCtx, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let indirect_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Buffer for storing multi draw information"),
            size: (MAX_CHUNKS * size_of::<DrawIndexedIndirect>()) as _,
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunks vertex buffer"),
            size: (MAX_VERTICES * size_of::<Vertex>()) as _,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunks index buffer"),
            size: (MAX_INDICES * size_of::<u32>()) as _,
            usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let chunk_loc_ssbo = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Chunk location SSBO"),
            size: (MAX_CHUNKS * size_of::<Vector3<f32>>()) as _,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let chunk_loc_ssbo_bind_group_layout = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let chunk_loc_ssbo_bind_group = ctx
            .device
            .create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &chunk_loc_ssbo_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: chunk_loc_ssbo.as_entire_binding(),
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
                bind_group_layouts: &[camera_bind_group_layout, &chunk_loc_ssbo_bind_group_layout],
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
            vertex_buffer,
            num_vertices: 0,
            index_buffer,
            num_indices: 0,
            indirect_buffer,
            num_indirect: 0,
            chunk_loc_ssbo,
            chunk_loc_ssbo_bind_group,
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

        // for index in &mut indices {
        //     *index += self.num_vertices as u32;
        // }

        let can_write_vertices = self.num_vertices + vertices.len() < MAX_VERTICES;
        let can_write_indices = self.num_indices + indices.len() < MAX_INDICES;
        let can_write_indirect = self.num_indirect + 1 < MAX_CHUNKS;

        if can_write_vertices && can_write_indices && can_write_indirect {
            info!(
                "Writing chunk #{}, currently we have {} verts",
                self.num_indirect, self.num_vertices
            );
            ctx.queue.write_buffer(
                &self.vertex_buffer,
                (self.num_vertices * size_of::<Vertex>()) as _,
                bytemuck::cast_slice(&vertices),
            );

            ctx.queue.write_buffer(
                &self.index_buffer,
                (self.num_indices * size_of::<u32>()) as _,
                bytemuck::cast_slice(&indices),
            );

            let indirect_buffer = DrawIndexedIndirect {
                vertex_count: indices.len() as u32,
                instance_count: 1,
                base_index: self.num_indices as u32,
                vertex_offset: self.num_vertices as i32,
                base_instance: 0,
            };
            ctx.queue.write_buffer(
                &self.indirect_buffer,
                (self.num_indirect * size_of::<DrawIndexedIndirect>()) as _,
                indirect_buffer.as_bytes(),
            );

            let chunk_location = [chunk_location];
            let chunk_location = bytemuck::cast_slice(&chunk_location);
            ctx.queue.write_buffer(
                &self.chunk_loc_ssbo,
                (self.num_indirect * size_of::<Vector3<f32>>()) as _,
                chunk_location,
            );

            self.num_vertices += vertices.len();
            self.num_indices += indices.len();
            self.num_indirect += 1;
        } else {
        }

        let renderer = ChunkRenderer {};

        self.renderers.insert(chunk_location, renderer);
    }
}

impl Renderer for ChunkRenderManager {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup, render_ctx: &RenderCtx) {
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.chunk_loc_ssbo_bind_group, &[]);

        // Push current chunk location
        // let loc = [position.to_world_location_f32()];
        // render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&loc));

        render_pass.multi_draw_indexed_indirect(&self.indirect_buffer, 0 as _, self.num_indirect as _);
        // render_pass.draw_indexed(0..self.num_indices as u32, 0, 0..1);
        // render_pass.draw_indexed_indirect(&self.indirect_buffer, 0);
    }
}

pub struct ChunkRenderer {}
