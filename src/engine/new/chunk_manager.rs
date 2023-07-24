use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use cgmath::Vector3;
use wgpu::{BindGroup, RenderPass};

use crate::engine::new::chunk::Chunk;
use crate::engine::new::location::ChunkLocation;
use crate::engine::new::meshing::ChunkMeshGenerator;
use crate::engine::new::worldgen::WorldGenerator;
use crate::engine::rendering::{RenderCtx, Renderer};

pub struct ChunkManager {
    chunks: HashMap<ChunkLocation, Chunk>,
    chunk_generator: WorldGenerator,
}

impl ChunkManager {
    pub fn new() -> Self {
        let chunk_generator = WorldGenerator::new(123);

        Self {
            chunks: HashMap::new(),
            chunk_generator,
        }
    }

    pub fn generate_all_chunks(&mut self) {
        self.generate_new(ChunkLocation::new(Vector3::new(0, 0, 0)));
        self.generate_new(ChunkLocation::new(Vector3::new(1, 0, 0)));
        self.generate_new(ChunkLocation::new(Vector3::new(0, 0, 1)));
        self.generate_new(ChunkLocation::new(Vector3::new(1, 0, 1)));
    }

    pub fn generate_all_chunk_meshes(&mut self, render_ctx: &Rc<RefCell<RenderCtx>>, camera_bind_group_layout: &wgpu::BindGroupLayout) {
        let mut queue = Vec::new();
        for (loc, chunk) in &mut self.chunks {
            if chunk.mesh.is_none() {
                queue.push(*loc);
            }
        }

        for loc in queue {
            let mesh = ChunkMeshGenerator::generate_mesh(render_ctx.clone(), camera_bind_group_layout, loc, &self.chunks);
            self.chunks
                .get_mut(&loc)
                .expect("Can not insert mesh into a non-existing chunk")
                .mesh = Some(mesh);
        }
    }

    fn generate_new(&mut self, location: ChunkLocation) {
        let chunk_data = self.chunk_generator.get_chunk_data_at(location);
        let chunk = Chunk::new(location, chunk_data);
        self.chunks.insert(location, chunk);
    }
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        self.chunks.iter().for_each(|(_, chunk)| {
            if let Some(renderer) = chunk.get_renderer() {
                renderer.render(render_pass, camera_bind_group);
            }
        })
    }
}
