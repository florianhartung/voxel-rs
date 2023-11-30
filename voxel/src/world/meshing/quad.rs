use cgmath::Vector3;

use crate::world::location::{LocalChunkLocation, WithinBounds};
use crate::world::meshing::direction::Direction;

#[derive(Debug)]
pub struct Quad {
    pub position: LocalChunkLocation<WithinBounds>,
    pub direction: Direction,
    pub data: FaceData,
    pub ambient_occlusion_values: [f32; 4],
    pub reversed_orientation: bool,
}

impl Quad {
    pub fn new(
        position: LocalChunkLocation<WithinBounds>,
        direction: Direction,
        data: FaceData,
        ao_values: [f32; 4],
        reversed_orientation: bool,
    ) -> Self {
        Self {
            position,
            direction,
            data,
            ambient_occlusion_values: ao_values,
            reversed_orientation,
        }
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
