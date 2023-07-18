use crate::engine::world::chunk::direction::Direction;
use crate::engine::world::chunk::local_location::{IndexedLocalLocation, LocalLocation};
use crate::engine::world::voxel::Voxel;

pub struct ChunkData {
    pub(crate) data: Box<[Voxel; super::CHUNK_SIZE.pow(3) as usize]>,
}

impl ChunkData {
    pub fn new_with_uniform_type(voxel: Voxel) -> Self {
        ChunkData {
            data: vec![voxel; super::CHUNK_SIZE.pow(3) as usize]
                .into_boxed_slice()
                .try_into()
                .expect(
                    "Size of boxed slice is expected to equal the size of ChunkData's data array",
                ),
        }
    }

    pub fn get_voxel_mut(&mut self, position: LocalLocation) -> &mut Voxel {
        &mut self.data[IndexedLocalLocation::from(position)]
    }

    pub fn get_voxel(&self, position: LocalLocation) -> &Voxel {
        &self.data[IndexedLocalLocation::from(position)]
    }

    pub fn get_neighboring_voxel(
        &self,
        pos: LocalLocation,
        direction: &Direction,
    ) -> Option<&Voxel> {
        let neighbor = pos + direction.to_vec();

        neighbor.map(|x| self.get_voxel(x))
    }
}
