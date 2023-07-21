use cgmath::{Vector3, Zero};
use strum::IntoEnumIterator;

use crate::engine::world::chunk::direction::Direction;
use crate::engine::world::chunk::local_location::LocalLocation;
use crate::engine::world::chunk::{Chunk, ChunkData};
use crate::engine::world::mesh::{Mesh, MeshVertex};

pub fn generate_mesh_from_chunk_data(data: &ChunkData) -> Mesh {
    let mesh = generate_culled_mesh(data);
    dbg!(mesh.vertices.len());

    mesh
}

pub fn generate_greedy_mesh(data: &ChunkData) -> Mesh {
    let mut quads: Vec<([Vector3<i32>; 4], Vector3<f32>)> = Vec::new();
    for d in 0..3 {
        let mut j: i32;
        let mut k: i32;
        let mut l: i32;
        let mut w: i32;
        let mut h: i32;
        let mut u = (d + 1) % 3;
        let mut v = (d + 2) % 3;
        let mut x: Vector3<i32> = Vector3::zero();
        let mut q: Vector3<i32> = Vector3::zero();
        let mut mask = vec![false; super::CHUNK_SIZE.pow(2) as usize].into_boxed_slice();
        q[d] = 1;

        x[d] = -1;
        while x[d] < super::CHUNK_SIZE as i32 {
            // Compute mask
            let mut n: i32 = 0;
            x[v] = 0;
            while x[v] < super::CHUNK_SIZE as i32 {
                x[u] = 0;
                while x[u] < super::CHUNK_SIZE as i32 {
                    mask[n as usize] = (if 0 <= x[d] {
                        data.get_voxel(
                            LocalLocation::try_from(x).expect("Should be inside boundaries"),
                        )
                        .ty != 0
                    } else {
                        false
                    }) != (if x[d] < (super::CHUNK_SIZE - 1) as i32 {
                        data.get_voxel(
                            LocalLocation::try_from(x + q).expect("Should be inside boundaries"),
                        )
                        .ty != 0
                    } else {
                        false
                    });

                    n += 1;
                    x[u] += 1;
                }
                x[v] += 1;
            }

            // Increment x[d]
            x[d] += 1;
            // Generate mesh for mask using lexicographic ordering
            n = 0;
            for j in 0..super::CHUNK_SIZE as i32 {
                let mut i = 0;
                while i < super::CHUNK_SIZE as i32 {
                    if mask[n as usize] {
                        // Compute width
                        w = 1;
                        while i + w < super::CHUNK_SIZE as i32 && mask[(n + w) as usize] {
                            w += 1;
                        }

                        let mut done = false;
                        let mut h: i32 = 1;
                        while j + h < super::CHUNK_SIZE as i32 {
                            let mut k = 0;
                            while k < w {
                                if !mask[(n + k + h * super::CHUNK_SIZE as i32) as usize] {
                                    done = true;
                                    break;
                                }

                                k += 1;
                            }
                            if done {
                                break;
                            }

                            h += 1;
                        }
                        // Add quad
                        x[u] = i;
                        x[v] = j;
                        let mut du: Vector3<i32> = Vector3::zero();
                        let mut dv: Vector3<i32> = Vector3::zero();
                        du[u] = w;
                        dv[v] = h;

                        quads.push((
                            [x, x + du, x + du + dv, x + dv].into(),
                            data.get_voxel(
                                LocalLocation::try_from(x).expect("Should be inside boundaries"),
                            )
                            .color(),
                        ));
                        // Zero-out mask
                        for l in 0..h {
                            for k in 0..w {
                                mask[(n + k + l * super::CHUNK_SIZE as i32) as usize] = false;
                            }
                        }
                        // Increment counters and continue
                        i += w;
                        n += w;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(Vec::new(), Vec::new());
    for q in quads {
        mesh.add_quad(
            q.0[0].cast().unwrap(),
            q.0[1].cast().unwrap(),
            q.0[2].cast().unwrap(),
            q.0[3].cast().unwrap(),
            q.1,
        );
    }

    mesh
}

pub fn generate_culled_mesh(data: &ChunkData) -> Mesh {
    fn is_solid(data: &ChunkData, coords: LocalLocation, offset: Vector3<i32>) -> bool {
        (coords + offset)
            .filter(|&x| data.get_voxel(x).ty != 0)
            .is_some()
    }

    let mut vertices = Vec::with_capacity(super::CHUNK_SIZE as usize);
    let mut indices: Vec<u32> = Vec::new();

    fn generate_quad<T: Into<Vector3<f32>>>(
        vertices: &mut Vec<MeshVertex>,
        indices: &mut Vec<u32>,
        pos: T,
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        color: Vector3<f32>,
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

        vertices.push(MeshVertex::new(a, color));
        vertices.push(MeshVertex::new(b, color));
        vertices.push(MeshVertex::new(c, color));
        vertices.push(MeshVertex::new(d, color));

        indices.extend([vertex_index + 0, vertex_index + 1, vertex_index + 2]);
        indices.extend([vertex_index + 2, vertex_index + 3, vertex_index + 0]);
    }

    LocalLocation::iter()
        .filter(|&pos| data.get_voxel(pos).ty >= 1)
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
                        data.get_voxel(pos).color(),
                    );
                }
            }
        });

    Mesh::new(vertices, indices)
}
