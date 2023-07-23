use cgmath::Vector3;

use crate::engine::world::chunk::local_location::LocalLocation;
use crate::engine::world::voxel::Voxel;

#[derive(Copy, Clone, Debug)]
pub struct VoxelFace {
    pub position: Vector3<u32>,
    pub voxel_type: u32,
}

impl VoxelFace {
    pub fn new(position: Vector3<u32>, voxel: &Voxel) -> Self {
        Self {
            position,
            voxel_type: voxel.ty as u32,
        }
    }
}

impl From<VoxelFace> for LocalLocation {
    fn from(value: VoxelFace) -> Self {
        LocalLocation::try_from(value.position).expect("Invalid voxel face outside of the chunk")
    }
}
