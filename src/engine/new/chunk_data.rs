use std::fmt::{Debug, Formatter};

use crate::engine::new::location::{LocalChunkLocation, OutsideBounds, WithinBounds};
use crate::engine::new::voxel_data::{VoxelData, VoxelType};
use crate::engine::new::CHUNK_SIZE;

pub struct ChunkData {
    voxels: Box<[VoxelData; CHUNK_SIZE.pow(3)]>,
}

impl Default for ChunkData {
    fn default() -> Self {
        ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air))
    }
}

impl Debug for ChunkData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChunkData")
    }
}

impl ChunkData {
    pub fn new_with_uniform_data(voxel_data: VoxelData) -> Self {
        Self {
            voxels: vec![voxel_data; CHUNK_SIZE.pow(3)]
                .into_boxed_slice()
                .try_into()
                .expect("Expected the vec size and the array size to be equal. Both should have a length of CHUNK_SIZE.pow(3)"),
        }
    }

    pub fn get_voxel(&self, local_chunk_location: LocalChunkLocation<WithinBounds>) -> &VoxelData {
        &self.voxels[Self::position_to_index(local_chunk_location)]
    }

    pub fn get_voxel_mut(&mut self, local_chunk_location: LocalChunkLocation<WithinBounds>) -> &mut VoxelData {
        &mut self.voxels[Self::position_to_index(local_chunk_location)]
    }

    pub fn try_get_voxel(&self, local_chunk_location: LocalChunkLocation<OutsideBounds>) -> Option<&VoxelData> {
        Some(&self.voxels[Self::position_to_index(local_chunk_location.try_into_checked()?)])
    }

    fn position_to_index(local_chunk_location: LocalChunkLocation<WithinBounds>) -> usize {
        local_chunk_location.z as usize * CHUNK_SIZE.pow(2) + local_chunk_location.y as usize * CHUNK_SIZE + local_chunk_location.x as usize
    }
}
