use crate::engine::world::voxel::Voxel;
use cgmath::Vector3;
use itertools::iproduct;
use std::ops::{Add, Deref, Index};
use std::slice::SliceIndex;

/// A valid location inside a chunk
#[derive(Copy, Clone, Debug)]
pub struct LocalLocation {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl LocalLocation {
    pub fn new(x: u32, y: u32, z: u32) -> Option<Self> {
        Self::is_valid(x, y, z).then(|| Self { x, y, z })
    }

    pub fn try_from<T: Into<Vector3<A>>, A: TryInto<u32>>(vec: T) -> Option<Self> {
        let vec = vec.into();

        Self::new(
            vec.x.try_into().ok()?,
            vec.y.try_into().ok()?,
            vec.z.try_into().ok()?,
        )
    }

    pub fn iter() -> impl Iterator<Item = LocalLocation> {
        iproduct!(
            0..super::CHUNK_SIZE,
            0..super::CHUNK_SIZE,
            0..super::CHUNK_SIZE
        )
        .map(|(x, y, z)| Self::new(x, y, z).expect("Expected a valid local location because it is generated using the chunk size as a boundary"))
    }

    fn is_valid(x: u32, y: u32, z: u32) -> bool {
        x < super::CHUNK_SIZE && y < super::CHUNK_SIZE && z < super::CHUNK_SIZE
    }
}

impl From<LocalLocation> for Vector3<u32> {
    fn from(value: LocalLocation) -> Vector3<u32> {
        Vector3::new(value.x, value.y, value.z)
    }
}

impl<T: Into<Vector3<i32>>> Add<T> for LocalLocation {
    type Output = Option<LocalLocation>;

    fn add(self, rhs: T) -> Self::Output {
        let rhs: Vector3<i32> = rhs.into();
        let lhs: Vector3<i32> = Vector3::from(self)
            .cast()
            .expect("Chunk is too big to fit into an i32");

        LocalLocation::try_from((lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z))
    }
}

const SIZE: usize = super::CHUNK_SIZE as usize;
const SIZE2: usize = super::CHUNK_SIZE.pow(2) as usize;

pub type IndexedLocalLocation = usize;
impl From<IndexedLocalLocation> for LocalLocation {
    fn from(value: IndexedLocalLocation) -> Self {
        let x = value % SIZE;
        let y = value / SIZE % SIZE;
        let z = value / SIZE2 % SIZE;
        LocalLocation::new(x as u32, y as u32, z as u32).expect("This is a valid local location because it is generated using an index from within the chunk size boundaries")
    }
}

impl From<LocalLocation> for IndexedLocalLocation {
    fn from(value: LocalLocation) -> Self {
        let (x, y, z) = (value.x as usize, value.y as usize, value.z as usize);

        let index = z * SIZE2 + y * SIZE + x;

        index
    }
}
