use cgmath::{Vector3, Zero};
use itertools::iproduct;
use strum::IntoEnumIterator;

use crate::engine::world::chunk::direction::Direction;
use crate::engine::world::chunk::local_location::LocalLocation;
use crate::engine::world::chunk::voxel_face::VoxelFace;
use crate::engine::world::chunk::ChunkData;
use crate::engine::world::mesh::{Mesh, MeshVertex};
use crate::engine::world::voxel::Voxel;

pub fn generate_mesh_from_chunk_data(data: &ChunkData) -> Mesh {
    let mesh = generate_greedy_mesh(data);

    mesh
}

pub fn generate_greedy_mesh(data: &ChunkData) -> Mesh {
    struct ChunkSize {
        width: u32,
        height: u32,
        length: u32,
    }

    const CHUNK_SIZE: ChunkSize = ChunkSize {
        width: super::CHUNK_SIZE,
        height: super::CHUNK_SIZE,
        length: super::CHUNK_SIZE,
    };

    let mut quads: Vec<([Vector3<i32>; 4], Vector3<f32>, bool)> = Vec::new(); // A vector with pairs of quads and a color for that quad. A quad itself is represented by a list of positions of length 4.

    for is_backface in [true, false] {
        // d is the dimension in which the faces which will be generated point. This is also the normal direction of the masks we are building
        for d in 0..3 {
            // The second dimension
            let mut u = (d + 1) % 3;

            // The third dimension
            let mut v = (d + 2) % 3;

            // The location of the current voxel
            let mut x: Vector3<i32> = Vector3::zero();

            // A vector that points into the direction of the next voxel.
            // The next voxel being in the direction defined by `d`.
            let mut q: Vector3<i32> = Vector3::zero();
            q[d] = 1;

            #[rustfmt::skip]
                let face_direction = match d {
                0 => if is_backface { Direction::XNeg } else { Direction::XPos },
                1 => if is_backface { Direction::YNeg } else { Direction::YPos },
                2 => if is_backface { Direction::ZNeg } else { Direction::ZPos },
                _ => unreachable!("d can only be in the range 0..3 because there are only 3 dimensions"),
            };

            // Move through the `d` dimension from front to back
            for i in -1..(CHUNK_SIZE.width as i32) {
                x[d] = i;

                // Compute mask
                // A mask for the current layer we are constructing the quad faces for.
                let mut mask: Vec<Option<VoxelFace>> = vec![None; (CHUNK_SIZE.width * CHUNK_SIZE.height) as usize];

                let mut n: i32 = 0;
                for i in 0..(CHUNK_SIZE.height as i32) {
                    x[v] = i;
                    for i in 0..(CHUNK_SIZE.width as i32) {
                        x[u] = i;

                        let voxel_face1 = (x[d] >= 0)
                            .then(|| {
                                let loc = LocalLocation::try_from(x).expect("Should be inside because of check before");
                                let voxel = data.get_voxel(loc);
                                (voxel.ty != 0).then(|| VoxelFace::new(loc.into(), voxel))
                            })
                            .flatten();

                        let voxel_face2 = (x[d] < (CHUNK_SIZE.width - 1) as i32)
                            .then(|| {
                                let loc =
                                    LocalLocation::try_from(x + q).expect("Should be inside because of check before and q is added to x");
                                let voxel = data.get_voxel(loc);
                                (voxel.ty != 0).then(|| VoxelFace::new(loc.into(), voxel))
                            })
                            .flatten();

                        mask[n as usize] = if voxel_face1.is_some()
                            && voxel_face2.is_some()
                            && voxel_face1.unwrap().voxel_type != 0
                            && voxel_face2.unwrap().voxel_type != 0
                        {
                            None
                        } else {
                            if is_backface {
                                voxel_face1
                            } else {
                                voxel_face2
                            }
                        };

                        n += 1;
                    }
                }

                // Increment x[d]
                x[d] += 1;

                // Generate mesh for mask
                n = 0;
                for j in 0..CHUNK_SIZE.height as i32 {
                    let mut i = 0;
                    while i < CHUNK_SIZE.width as i32 {
                        if let Some(face) = mask[n as usize] {
                            // Compute the quad's width
                            let mut width = 1;
                            while i + width < super::CHUNK_SIZE as i32
                                && mask[(n + width) as usize]
                                    .map(|f| face.voxel_type == f.voxel_type)
                                    .unwrap_or(false)
                            {
                                width += 1;
                            }

                            // Compute the quad's height
                            let mut height: i32 = 1;
                            'height_loop: while j + height < CHUNK_SIZE.height as i32 {
                                for k in 0..width {
                                    if mask[(n + k + height * CHUNK_SIZE.width as i32) as usize]
                                        .filter(|f| f.voxel_type == face.voxel_type)
                                        .is_none()
                                    {
                                        break 'height_loop;
                                    }
                                }
                                height += 1;
                            }

                            // Add quad to quads list
                            x[u] = i;
                            x[v] = j;
                            let mut du: Vector3<i32> = Vector3::zero();
                            let mut dv: Vector3<i32> = Vector3::zero();
                            du[u] = width;
                            dv[v] = height;
                            quads.push((
                                [x, x + du, x + du + dv, x + dv].into(),
                                Voxel::new(face.voxel_type as u8).color(),
                                is_backface,
                            ));

                            // Zero-out mask
                            for (l, k) in iproduct!(0..height, 0..width) {
                                mask[(n + k + l * super::CHUNK_SIZE as i32) as usize] = None;
                            }

                            // Increment counters and continue
                            i += width;
                            n += width;
                        } else {
                            i += 1;
                            n += 1;
                        }
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(Vec::new(), Vec::new());

    for q in &quads {
        mesh.add_quad(
            q.0[0].cast().unwrap(),
            q.0[1].cast().unwrap(),
            q.0[2].cast().unwrap(),
            q.0[3].cast().unwrap(),
            q.1,
            q.2,
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

fn are_faces_combinable(data: &ChunkData, face1: VoxelFace, face2: VoxelFace) -> bool {
    let voxel1 = data.get_voxel(face1.into());
    let voxel2 = data.get_voxel(face2.into());

    voxel1.ty == voxel2.ty
}
