use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation};
use crate::engine::world::mesh::{Mesh, MeshRenderer};
use crate::engine::world::voxel_data::VoxelType;

#[derive(Debug)]
pub struct Chunk {
    pub location: ChunkLocation,
    pub data: ChunkData,
    pub mesh: ChunkMesh,
    pub is_empty: bool,
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
}

impl Chunk {
    pub fn new(location: ChunkLocation, data: ChunkData) -> Self {
        let contains_non_air_voxels = LocalChunkLocation::iter().any(|loc| data.get_voxel(loc).ty != VoxelType::Air);

        Self {
            location,
            data,
            mesh: ChunkMesh::None,
            is_empty: !contains_non_air_voxels,
        }
    }

    pub fn get_renderer(&self, render_empty: bool) -> Option<&MeshRenderer> {
        if let ChunkMesh::Generated(mesh) = &self.mesh {
            Some(mesh.get_renderer())
        } else if let ChunkMesh::Empty(mesh) = &self.mesh {
            if render_empty {
                Some(mesh.get_renderer())
            } else {
                None
            }
        } else {
            None
        }
    }
}
