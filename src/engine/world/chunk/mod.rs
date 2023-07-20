use cgmath::{Vector3, Zero};
use std::ops::{Add, Deref, DerefMut};

use strum::IntoEnumIterator;

use crate::engine::rendering::RenderCtx;
use crate::engine::world::chunk::data::ChunkData;
use crate::engine::world::chunk::local_location::LocalLocation;
use crate::engine::world::chunk::renderer::ChunkRenderer;
use crate::engine::world::mesh::Mesh;

pub mod data;
pub mod direction;
pub mod local_location;
pub mod meshing;
pub mod renderer;

pub const CHUNK_SIZE: u32 = 64;

pub struct Chunk {
    data: ChunkData,
    position: Vector3<u32>,
}

impl Chunk {
    pub fn new(chunk_data: ChunkData, position: Vector3<u32>) -> Self {
        Self {
            data: chunk_data,
            position,
        }
    }

    pub fn into_meshed(self) -> MeshedChunk {
        let mesh = meshing::generate_mesh_from_chunk_data(&self.data);

        MeshedChunk {
            data: self.data,
            mesh,
            position: self.position,
        }
    }
}

pub struct MeshedChunk {
    data: ChunkData,
    mesh: Mesh,
    position: Vector3<u32>,
}

impl MeshedChunk {
    pub fn get_renderer(
        &self,
        render_ctx: &RenderCtx,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> ChunkRenderer {
        ChunkRenderer {
            mesh_renderer: self.mesh.get_renderer(
                render_ctx,
                camera_bind_group_layout,
                (self.position * CHUNK_SIZE)
                    .cast()
                    .expect("u32 should fit into f32"),
            ),
        }
    }

    pub fn randomize_data(&mut self) {
        LocalLocation::iter().for_each(|pos| {
            self.data.get_voxel_mut(pos).ty = if fastrand::f32() < 0.5 { 1 } else { 0 }
        });

        self.mesh = meshing::generate_mesh_from_chunk_data(&self.data);
    }

    pub fn update_renderer(&self, chunk_renderer: &mut ChunkRenderer, render_ctx: &RenderCtx) {
        self.mesh
            .update_renderer(&mut chunk_renderer.mesh_renderer, render_ctx);
    }
}
