use std::ops::Add;

use cgmath::{ElementWise, Matrix3, Vector3, Zero};
use itertools::iproduct;
use strum::IntoEnumIterator;

use crate::engine::world::chunk::local_coordinates::LocalCoordinates;
use crate::engine::world::chunk::{Chunk, ChunkData, Direction};
use crate::engine::world::mesh::{Mesh, MeshVertex};

pub fn generate_mesh_from_chunk_data(data: &ChunkData) -> Mesh {
    let vertices_per_direction = Chunk::SIZE + 1;

    let mut vertices = Vec::with_capacity(Chunk::SIZE3 as usize);

    iproduct!(
        0..vertices_per_direction,
        0..vertices_per_direction,
        0..vertices_per_direction
    )
    .map(|(x, y, z)| MeshVertex::from_pos([x as f32, y as f32, z as f32]))
    .for_each(|vertex| vertices.push(vertex));

    let mut indices: Vec<u32> = Vec::new();

    let mut add_tri = |pos: LocalCoordinates, relative: [Vector3<i16>; 3]| {
        relative
            .into_iter()
            .map(|x| pos + x)
            .map(|x| x.to_index(vertices_per_direction))
            .for_each(|x| indices.push(x));
    };

    LocalCoordinates::iter()
        .map(|pos| (pos, data.get_voxel(&pos)))
        .filter(|(_pos, voxel)| voxel.ty == 1)
        .for_each(|(pos, _voxel)| {
            for dir in Direction::iter() {
                let is_neg = match dir {
                    Direction::XPos | Direction::YPos | Direction::ZPos => false,
                    Direction::XNeg | Direction::YNeg | Direction::ZNeg => true,
                };

                let mut dir_vec = dir
                    .to_vec()
                    .cast::<f32>()
                    .expect("Failed to cast direction vector");

                // Calculate absolute vector
                dir_vec.mul_assign_element_wise(dir_vec);

                // Matrix that shifts the components of a vector
                let shift_mat: Matrix3<f32> =
                    Matrix3::from([[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);

                let c1 = dir_vec;
                let c2 = shift_mat * dir_vec;
                let c3 = shift_mat * shift_mat * dir_vec;

                let mut c1 = c1.cast::<i16>().unwrap();
                let c2 = c2.cast::<i16>().unwrap();
                let c3 = c3.cast::<i16>().unwrap();

                // In the negative directions c1 has to be ignored when adding the triangles
                if is_neg {
                    c1 = Vector3::zero();
                }

                add_tri(pos, [c1, c1 + c2, c1 + c3]);
                add_tri(pos, [c1 + c2 + c3, c1 + c2, c1 + c3]);
            }
        });

    Mesh::new(vertices, indices)
}
