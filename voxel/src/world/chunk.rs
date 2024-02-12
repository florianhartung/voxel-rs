use crate::world::chunk_data::ChunkData;
use crate::world::location::ChunkLocation;
use crate::world::mesh::{Mesh, MeshRenderer};

#[derive(Debug)]
pub struct Chunk {
    pub location: ChunkLocation,
    pub data: ChunkData,
    pub mesh: ChunkMesh,

    /// Uniform resources for chunk position uniform
    position_buffer: wgpu::Buffer,
    pub position_bind_group_layout: wgpu::BindGroupLayout,
    pub position_bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub enum ChunkMesh {
    None,
    Generated(Mesh),
    Empty(Mesh),
}

impl ChunkMesh {
    pub fn new(mesh: Mesh) -> Self {
        if !mesh.indices.is_empty() {
            Self::Generated(mesh)
        } else {
            Self::Empty(mesh)
        }
    }

    pub fn get_renderer(&self, render_empty: bool) -> Option<&MeshRenderer> {
        match &self {
            Self::None => None,
            Self::Generated(mesh) => Some(mesh.get_renderer()),
            Self::Empty(mesh) => render_empty.then(|| mesh.get_renderer()),
        }
    }
}
