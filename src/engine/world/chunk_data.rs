use std::fmt::{Debug, Formatter};

use crate::engine::world::chunk_data::ChunkData::{UniformType, Voxels};
use crate::engine::world::location::{LocalChunkLocation, OutsideBounds, WithinBounds};
use crate::engine::world::voxel_data::{VoxelData, VoxelType};
use crate::engine::world::CHUNK_SIZE;

pub enum ChunkData {
    Voxels(Box<[VoxelData; CHUNK_SIZE.pow(3)]>),
    UniformType(VoxelData),
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
    pub fn new_filled_with_uniform_data(voxel_data: VoxelData) -> Self {
        Voxels(
            vec![voxel_data; CHUNK_SIZE.pow(3)]
                .into_boxed_slice()
                .try_into()
                .expect("Expected the vec size and the array size to be equal. Both should have a length of CHUNK_SIZE.pow(3)"),
        )
    }

    pub fn new_with_uniform_data(voxel_data: VoxelData) -> Self {
        UniformType(voxel_data)
    }

    pub fn try_convert_into_uniform(&mut self) {
        if matches!(self, UniformType(_)) {
            return;
        }

        let mut uniform_data = None;
        for loc in LocalChunkLocation::iter() {
            if let Some(a) = uniform_data {
                if a != self.get_voxel(loc) {
                    return;
                }
            } else {
                uniform_data = Some(self.get_voxel(loc));
            }
        }

        if let Some(data) = uniform_data {
            *self = Self::new_with_uniform_data(*data);
        }
    }

    pub fn get_voxel(&self, local_chunk_location: LocalChunkLocation<WithinBounds>) -> &VoxelData {
        match self {
            Voxels(data) => &data[Self::position_to_index(local_chunk_location)],
            UniformType(voxel_data) => voxel_data,
        }
    }

    pub fn set_voxel_data(&mut self, local_chunk_location: LocalChunkLocation<WithinBounds>, new_voxel_data: VoxelData) {
        match self {
            Voxels(data) => data[Self::position_to_index(local_chunk_location)] = new_voxel_data,
            UniformType(uniform_data) => {
                if *uniform_data == new_voxel_data {
                    return;
                }

                *self = Self::new_filled_with_uniform_data(*uniform_data);

                match self {
                    Voxels(data) => data[Self::position_to_index(local_chunk_location)] = new_voxel_data,
                    UniformType(_) => unreachable!(),
                }
            }
        }
    }

    pub fn try_get_voxel(&self, local_chunk_location: LocalChunkLocation<OutsideBounds>) -> Option<&VoxelData> {
        Some(self.get_voxel(local_chunk_location.try_into_checked()?))
    }

    fn position_to_index(local_chunk_location: LocalChunkLocation<WithinBounds>) -> usize {
        local_chunk_location.z as usize * CHUNK_SIZE.pow(2) + local_chunk_location.y as usize * CHUNK_SIZE + local_chunk_location.x as usize
    }
}
