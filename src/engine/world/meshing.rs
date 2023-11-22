use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::{Neg, Range};
use std::rc::Rc;

use cgmath::prelude::*;
use cgmath::Vector3;
use fastrand::Rng;
use strum::IntoEnumIterator;

use crate::engine::rendering::RenderCtx;
use crate::engine::vector_utils::{AbsValue, RemEuclid};
use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation, WorldLocation};
use crate::engine::world::mesh::{Mesh, Vertex};
use crate::engine::world::meshing::direction::Direction;
use crate::engine::world::meshing::quad::{FaceData, Quad};
use crate::engine::world::voxel_data::VoxelType;
use crate::engine::world::CHUNK_SIZE;

pub mod direction;
pub mod quad;

pub struct ChunkMeshGenerator {
    quads: Vec<Quad>,
}

impl ChunkMeshGenerator {
    pub fn generate_mesh_from_quads(
        chunk_location: ChunkLocation,
        quads: Vec<Quad>,
        render_ctx: Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Mesh {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        quads.iter().for_each(|quad| {
            let base_index = vertices.len() as u32;

            let mut pos = quad.position.to_f32() + chunk_location.to_world_location_f32();
            let direction = quad
                .direction
                .to_vec()
                .cast::<f32>()
                .expect("Conversion from i32 to f32 is safe")
                .abs();

            let (axis1, axis2) = quad.direction.get_normal_axes();
            let (axis1, axis2) = (axis1.cast::<f32>().unwrap().abs(), axis2.cast::<f32>().unwrap().abs());

            let is_backside = match quad.direction {
                Direction::XPos | Direction::YPos | Direction::ZPos => false,
                Direction::XNeg | Direction::YNeg | Direction::ZNeg => true,
            };

            if !is_backside {
                pos += direction;
            }

            vertices.push(Vertex::new(pos, quad.data.color, direction, quad.ambient_occlusion_values[0]));
            vertices.push(Vertex::new(
                pos + axis1,
                quad.data.color,
                direction,
                quad.ambient_occlusion_values[1],
            ));
            vertices.push(Vertex::new(
                pos + axis2,
                quad.data.color,
                direction,
                quad.ambient_occlusion_values[2],
            ));
            vertices.push(Vertex::new(
                pos + axis1 + axis2,
                quad.data.color,
                direction,
                quad.ambient_occlusion_values[3],
            ));

            {
                if is_backside && quad.reversed_orientation {
                    [0, 1, 2, 2, 1, 3]
                } else if is_backside && !quad.reversed_orientation {
                    [0, 1, 3, 3, 2, 0]
                } else if !is_backside && quad.reversed_orientation {
                    [2, 1, 0, 3, 1, 2]
                } else {
                    [2, 3, 0, 0, 3, 1]
                }
            }
            .iter()
            .for_each(|i| indices.push(base_index + i));
        });

        Mesh::new(render_ctx, camera_bind_group_layout, vertices, indices)
    }
    pub fn generate_mesh(
        render_ctx: Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        location: ChunkLocation,
        chunks: &hashbrown::HashMap<ChunkLocation, ChunkData>,
    ) -> Mesh {
        let quads = Self::generate_culled_mesh(
            location,
            &chunks
                .get(&location)
                .expect("Can't generate a mesh for a chunk that does not exist"),
            chunks,
        );

        Self::generate_mesh_from_quads(location, quads, render_ctx, camera_bind_group_layout)
    }

    pub fn generate_culled_mesh(
        current_location: ChunkLocation,
        data: &ChunkData,
        all_chunks: &hashbrown::HashMap<ChunkLocation, ChunkData>,
    ) -> Vec<Quad> {
        let mut quads = Vec::new();

        LocalChunkLocation::iter()
            .filter(|&pos| data.get_voxel(pos).ty != VoxelType::Air)
            .for_each(|pos| {
                for dir in Direction::iter() {
                    let neighbor_voxel_location = pos + dir;
                    let (mut axis1, mut axis2) = dir.get_normal_axes();
                    axis1 = axis1.abs();
                    axis2 = axis2.abs();

                    let get_voxel_in_world = |mut local_location: LocalChunkLocation| {
                        if let Some(within_current_chunk) = local_location.try_into_checked() {
                            data.get_voxel(within_current_chunk)
                        } else {
                            let mut chunk_loc = current_location;
                            if local_location.x < 0 {
                                local_location.x += CHUNK_SIZE as i32;
                                chunk_loc.x -= 1;
                            } else if local_location.x >= CHUNK_SIZE as i32 {
                                local_location.x -= CHUNK_SIZE as i32;
                                chunk_loc.x += 1;
                            }

                            if local_location.y < 0 {
                                local_location.y += CHUNK_SIZE as i32;
                                chunk_loc.y -= 1;
                            } else if local_location.y >= CHUNK_SIZE as i32 {
                                local_location.y -= CHUNK_SIZE as i32;
                                chunk_loc.y += 1;
                            }

                            if local_location.z < 0 {
                                local_location.z += CHUNK_SIZE as i32;
                                chunk_loc.z -= 1;
                            } else if local_location.z >= CHUNK_SIZE as i32 {
                                local_location.z -= CHUNK_SIZE as i32;
                                chunk_loc.z += 1;
                            }

                            all_chunks
                                .get(&chunk_loc)
                                .expect("Chunk not generated yet")
                                .get_voxel(
                                    local_location
                                        .try_into_checked()
                                        .expect("This should be a valid local location because the voxel offset is max 1"),
                                )
                        }
                    };

                    let calc_ao = |dir1: Vector3<i32>, dir2: Vector3<i32>| {
                        let s1 = get_voxel_in_world(neighbor_voxel_location + dir1).ty != VoxelType::Air;
                        let s2 = get_voxel_in_world(neighbor_voxel_location + dir2).ty != VoxelType::Air;
                        let c = get_voxel_in_world(neighbor_voxel_location + dir1 + dir2).ty != VoxelType::Air;

                        if s1 && s2 {
                            0.0
                        } else {
                            3.0 - (if s1 { 1.0 } else { 0.0 } + if s2 { 1.0 } else { 0.0 } + if c { 1.0 } else { 0.0 })
                        }
                    };

                    let ao_1 = calc_ao(axis1.neg(), axis2.neg());
                    let ao_2 = calc_ao(axis1, axis2.neg());
                    let ao_3 = calc_ao(axis1.neg(), axis2);
                    let ao_4 = calc_ao(axis1, axis2);

                    let reverse_quad_orientation = ao_1 + ao_4 <= ao_2 + ao_3;
                    // let reverse_quad_orientation = false;

                    let quad = Quad::new(
                        pos,
                        dir,
                        FaceData::new(voxel_type_to_color(
                            data.get_voxel(pos).ty,
                            WorldLocation::new(current_location, pos.into_unknown()),
                        )),
                        [ao_1, ao_2, ao_3, ao_4],
                        reverse_quad_orientation,
                    );

                    if let Some(same_chunk_neighbor) = neighbor_voxel_location.try_into_checked() {
                        if data.get_voxel(same_chunk_neighbor).ty == VoxelType::Air {
                            quads.push(quad);
                        }
                    } else if let Some(chunk) = all_chunks.get(&ChunkLocation::new(*current_location + dir.to_vec())) {
                        let neighbor_local = LocalChunkLocation::new(neighbor_voxel_location.rem_euclid(CHUNK_SIZE as i32))
                            .try_into_checked()
                            .expect("aa");

                        if chunk.get_voxel(neighbor_local).ty == VoxelType::Air {
                            quads.push(quad);
                        }
                    } else {
                        eprintln!("Neighbor chunk's data is not generated yet.")
                    }
                }
            });

        quads
    }
}

fn voxel_type_to_color(ty: VoxelType, voxel_position: WorldLocation) -> Vector3<f32> {
    let mut hasher = DefaultHasher::new();
    voxel_position.0.hash(&mut hasher);
    let mut rng = Rng::with_seed(hasher.finish());

    match ty {
        VoxelType::Air => Vector3::new(1.0, 0.0, 1.0),
        VoxelType::Dirt => Vector3::new(rand(&mut rng, 0.12..0.18), rand(&mut rng, 0.06..0.14), 0.02),
        VoxelType::Grass => Vector3::new(rand(&mut rng, 0.07..0.11), rand(&mut rng, 0.28..0.32), rand(&mut rng, 0.01..0.04)),
        VoxelType::Stone => v(rand(&mut rng, 0.25..0.35)),
    }
}

#[inline]
fn v(f: f32) -> Vector3<f32> {
    Vector3::new(f, f, f)
}
#[inline]
fn rand(rng: &mut Rng, range: Range<f32>) -> f32 {
    rng.f32() * (range.end - range.start) + range.start
}
