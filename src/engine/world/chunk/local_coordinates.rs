use std::ops::Add;

use cgmath::Vector3;
use itertools::iproduct;

use crate::engine::world::chunk::Chunk;

#[derive(Copy, Clone, Debug)]
pub struct LocalCoordinates {
    pub coords: Vector3<u8>,
}

impl From<Vector3<u8>> for LocalCoordinates {
    fn from(coords: Vector3<u8>) -> Self {
        Self { coords }
    }
}

impl LocalCoordinates {
    pub fn iter() -> impl Iterator<Item = LocalCoordinates> {
        iproduct!(0..Chunk::SIZE, 0..Chunk::SIZE, 0..Chunk::SIZE)
            .map(|(x, y, z)| Self::from(Vector3::new(x, y, z)))
    }

    pub fn is_out_of_bounds(&self) -> bool {
        !(0..Chunk::SIZE).contains(&self.coords.x)
            || !(0..Chunk::SIZE).contains(&self.coords.y)
            || !(0..Chunk::SIZE).contains(&self.coords.z)
    }

    pub fn to_index(&self, chunk_size: u8) -> u32 {
        (self.coords.x as u32) * (chunk_size as u32).pow(2)
            + (self.coords.y as u32) * (chunk_size as u32)
            + (self.coords.z as u32)
    }
}

impl Add<Self> for LocalCoordinates {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        (self.coords + rhs.coords).into()
    }
}

impl Add<Vector3<u8>> for LocalCoordinates {
    type Output = Self;

    fn add(self, rhs: Vector3<u8>) -> Self::Output {
        (self.coords + rhs).into()
    }
}

impl Add<(u8, u8, u8)> for LocalCoordinates {
    type Output = Self;

    fn add(self, rhs: (u8, u8, u8)) -> Self::Output {
        (self.coords + Vector3::from(rhs)).into()
    }
}

impl Add<Vector3<i16>> for LocalCoordinates {
    type Output = Self;

    fn add(self, rhs: Vector3<i16>) -> Self::Output {
        (self
            .coords
            .cast::<i16>()
            .expect("Invalid local coordinates")
            + rhs)
            .cast::<u8>()
            .expect("Invalid local coordinates")
            .into()
    }
}
