use cgmath::Vector3;

use crate::engine::world::location::{LocalChunkLocation, WithinBounds};
use crate::engine::world::meshing::direction::Direction;

#[derive(Debug)]
pub struct Quad {
    pub position: LocalChunkLocation<WithinBounds>,
    pub direction: Direction,
    pub data: FaceData,
}

impl Quad {
    pub fn new(position: LocalChunkLocation<WithinBounds>, direction: Direction, data: FaceData) -> Self {
        Self { position, direction, data }
    }
}

#[derive(Debug)]
pub struct FaceData {
    pub color: Vector3<f32>,
}

impl FaceData {
    pub fn new(color: Vector3<f32>) -> Self {
        Self { color }
    }
}
