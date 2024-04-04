use std::collections::HashMap;

use wgpu::{BindGroup, BindGroupLayout, RenderPass};

use crate::rendering::{RenderCtx, Renderer};
use crate::world::chunk_data::ChunkData;
use crate::world::location::ChunkLocation;
use crate::world::mesh::Mesh;
use crate::world::meshing::quad::Quad;
use crate::world::meshing::{ChunkMeshGenerator, NeighborChunks};

pub struct ChunkRenderManager {
    renderers: HashMap<ChunkLocation, ChunkRenderer>,
}

impl ChunkRenderManager {
    pub fn new() -> Self {
        Self { renderers: HashMap::new() }
    }

    pub fn generate_mesh(chunk_data: &ChunkData, neighbor_chunks: NeighborChunks) -> Vec<Quad> {
        ChunkMeshGenerator::generate_culled_mesh(chunk_data, neighbor_chunks)
    }

    pub fn save_mesh(
        &mut self,
        quads: Vec<Quad>,
        chunk_location: ChunkLocation,
        ctx: &RenderCtx,
        camera_bind_group_layout: &BindGroupLayout,
    ) {
        let mesh = ChunkMeshGenerator::generate_mesh_from_quads(chunk_location, quads, ctx, camera_bind_group_layout);

        self.renderers
            .insert(chunk_location, ChunkRenderer { mesh });
    }
}

impl Renderer for ChunkRenderManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        for renderer in self.renderers.values() {
            renderer
                .mesh
                .get_renderer()
                .render(render_pass, camera_bind_group);
        }
    }
}

pub struct ChunkRenderer {
    mesh: Mesh,
}
