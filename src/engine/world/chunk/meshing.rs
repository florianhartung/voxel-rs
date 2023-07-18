use cgmath::{ElementWise, Matrix3, Vector3, Zero};
use itertools::iproduct;
use strum::IntoEnumIterator;

use crate::engine::world::chunk::direction::Direction;
use crate::engine::world::chunk::local_location::LocalLocation;
use crate::engine::world::chunk::{Chunk, ChunkData};
use crate::engine::world::mesh::{Mesh, MeshVertex};

pub fn generate_mesh_from_chunk_data(data: &ChunkData) -> Mesh {
    fn is_solid(data: &ChunkData, coords: LocalLocation, offset: Vector3<i32>) -> bool {
        (coords + offset)
            .filter(|&x| data.get_voxel(x).ty != 0)
            .is_some()
    }

    let mut vertices = Vec::with_capacity(Chunk::SIZE3 as usize);
    let mut indices: Vec<u32> = Vec::new();

    fn generate_quad<T: Into<Vector3<f32>>>(
        vertices: &mut Vec<MeshVertex>,
        indices: &mut Vec<u32>,
        pos: T,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    ) {
        const VERTEX_POSITIONS_OFFSETS: [Vector3<f32>; 8] = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ];

        let pos = pos.into();

        let a = pos + VERTEX_POSITIONS_OFFSETS[a];
        let b = pos + VERTEX_POSITIONS_OFFSETS[b];
        let c = pos + VERTEX_POSITIONS_OFFSETS[c];
        let d = pos + VERTEX_POSITIONS_OFFSETS[d];

        let vertex_index = vertices.len() as u32;

        vertices.push(MeshVertex::from_pos(a));
        vertices.push(MeshVertex::from_pos(b));
        vertices.push(MeshVertex::from_pos(c));
        vertices.push(MeshVertex::from_pos(d));

        indices.extend([vertex_index + 0, vertex_index + 1, vertex_index + 2]);
        indices.extend([vertex_index + 2, vertex_index + 3, vertex_index + 0]);
    }

    LocalLocation::iter()
        .filter(|&pos| data.get_voxel(pos).ty == 1)
        .for_each(|pos| {
            let floating_pos = Vector3::from(pos).cast::<f32>().unwrap();

            for dir in Direction::iter() {
                let quad_indices = match dir {
                    Direction::XPos => [7, 5, 1, 3],
                    Direction::XNeg => [2, 0, 4, 6],
                    Direction::YPos => [2, 6, 7, 3],
                    Direction::YNeg => [0, 1, 5, 4],
                    Direction::ZPos => [6, 4, 5, 7],
                    Direction::ZNeg => [3, 1, 0, 2],
                };

                if !is_solid(data, pos, dir.to_vec()) {
                    generate_quad(
                        &mut vertices,
                        &mut indices,
                        floating_pos,
                        quad_indices[0],
                        quad_indices[1],
                        quad_indices[2],
                        quad_indices[3],
                    );
                }
            }
        });

    Mesh::new(vertices, indices)
}
