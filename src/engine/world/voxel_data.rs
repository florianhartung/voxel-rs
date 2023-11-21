#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VoxelData {
    pub ty: VoxelType,
}

impl VoxelData {
    pub const fn new(ty: VoxelType) -> Self {
        Self { ty }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VoxelType {
    Air,
    Dirt,
    Grass,
    Stone,
}
