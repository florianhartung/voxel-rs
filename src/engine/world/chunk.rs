use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation};
use crate::engine::world::mesh::{Mesh, MeshRenderer};
use crate::engine::world::voxel_data::VoxelType;

#[derive(Debug)]
pub struct Chunk {
    pub location: ChunkLocation,
    pub data: ChunkData,
    pub mesh: Option<Mesh>,
    pub is_empty: bool,
}

impl Chunk {
    pub fn new(location: ChunkLocation, data: ChunkData) -> Self {
        let contains_non_air_voxels = LocalChunkLocation::iter().any(|loc| data.get_voxel(loc).ty != VoxelType::Air);

        Self {
            location,
            data,
            mesh: None,
            is_empty: !contains_non_air_voxels,
        }
    }

    pub fn get_renderer(&self, render_empty_chunks: bool) -> Option<&MeshRenderer> {
        if render_empty_chunks || !self.is_empty {
            self.mesh.as_ref().map(|mesh| mesh.get_renderer())
        } else {
            None
        }
    }
}
