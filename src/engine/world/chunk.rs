use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::ChunkLocation;
use crate::engine::world::mesh::{Mesh, MeshRenderer};

#[derive(Debug)]
pub struct Chunk {
    pub location: ChunkLocation,
    pub data: ChunkData,
    pub mesh: Option<Mesh>,
}

impl Chunk {
    pub fn new(location: ChunkLocation, data: ChunkData) -> Self {
        Self {
            location,
            data,
            mesh: None,
        }
    }

    pub fn get_renderer(&self) -> Option<&MeshRenderer> {
        self.mesh.as_ref().map(|mesh| mesh.get_renderer())
    }
}
