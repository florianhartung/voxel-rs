use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, Neg, Range};

use anyhow::Context;
use anyhow::Result;
use cgmath::prelude::*;
use cgmath::Vector3;
use enum_map::EnumMap;
use fastrand::Rng;
use itertools::iproduct;
use lazy_static::lazy_static;
use strum::IntoEnumIterator;

use crate::rendering::RenderCtx;
use crate::vector_utils::{AbsValue, RemEuclid};
use crate::world::chunk_data::ChunkData;
use crate::world::location::{ChunkLocation, LocalChunkLocation, WithinBounds, WorldLocation};
use crate::world::mesh::{Mesh, Vertex};
use crate::world::meshing::direction::Direction;
use crate::world::meshing::quad::{FaceData, Quad};
use crate::world::voxel_data::VoxelType;
use crate::world::CHUNK_SIZE;

pub mod direction;
pub mod quad;

pub struct ChunkMeshGenerator {
    quads: Vec<Quad>,
}

impl ChunkMeshGenerator {
    pub fn generate_mesh_from_quads(
        chunk_location: ChunkLocation,
        quads: Vec<Quad>,
        render_ctx: impl Deref<Target = RenderCtx>,
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

    pub fn generate_culled_mesh(current_location: ChunkLocation, data: &ChunkData, neighbor_chunks: NeighborChunks) -> Vec<Quad> {
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
                            let mut relative_chunk_loc = ChunkLocation::new(Vector3::<i32>::zero());

                            if local_location.x < 0 {
                                local_location.x += CHUNK_SIZE as i32;
                                relative_chunk_loc.x -= 1;
                            } else if local_location.x >= CHUNK_SIZE as i32 {
                                local_location.x -= CHUNK_SIZE as i32;
                                relative_chunk_loc.x += 1;
                            }

                            if local_location.y < 0 {
                                local_location.y += CHUNK_SIZE as i32;
                                relative_chunk_loc.y -= 1;
                            } else if local_location.y >= CHUNK_SIZE as i32 {
                                local_location.y -= CHUNK_SIZE as i32;
                                relative_chunk_loc.y += 1;
                            }

                            if local_location.z < 0 {
                                local_location.z += CHUNK_SIZE as i32;
                                relative_chunk_loc.z -= 1;
                            } else if local_location.z >= CHUNK_SIZE as i32 {
                                local_location.z -= CHUNK_SIZE as i32;
                                relative_chunk_loc.z += 1;
                            }

                            neighbor_chunks.get(relative_chunk_loc).get_voxel(
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
                        FaceData::new(voxel_type_to_color_lookup(data.get_voxel(pos).ty, &pos)),
                        [ao_1, ao_2, ao_3, ao_4],
                        reverse_quad_orientation,
                    );

                    if let Some(same_chunk_neighbor) = neighbor_voxel_location.try_into_checked() {
                        if data.get_voxel(same_chunk_neighbor).ty == VoxelType::Air {
                            quads.push(quad);
                        }
                    } else {
                        let chunk = neighbor_chunks.get(ChunkLocation::new(dir.to_vec()));

                        let neighbor_local = LocalChunkLocation::new(neighbor_voxel_location.rem_euclid(CHUNK_SIZE as i32))
                            .try_into_checked()
                            .expect("aa");

                        if chunk.get_voxel(neighbor_local).ty == VoxelType::Air {
                            quads.push(quad);
                        }
                    }
                }
            });

        quads
    }
}

pub struct NeighborChunks<'a> {
    pub chunk_data: [&'a ChunkData; 27],
}

impl<'a> NeighborChunks<'a> {
    pub fn new<'b: 'a, F: Fn(&ChunkLocation) -> Option<&'b ChunkData> + 'b>(around: &ChunkLocation, get_chunk: F) -> Result<Self> {
        let mut v: [Option<&'b ChunkData>; 27] = [None; 27];

        iproduct!(-1i32..=1, -1..=1, -1..=1).for_each(|(dx, dy, dz)| {
            let current_location: ChunkLocation = *around + ChunkLocation::new(Vector3::<i32>::new(dx, dy, dz));

            let idx = (dx + 1) * 9 + (dy + 1) * 3 + (dz + 1);
            let Some(x) = get_chunk(&current_location) else {
                panic!(
                    "Failed to get chunk at {:?} which is relative by {:?} to the center chunk {:?}",
                    current_location,
                    Vector3::new(dx, dy, dz),
                    around
                );
            };
            v[idx as usize] = Some(x);
        });

        let chunk_data: Vec<&ChunkData> = v
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .expect("all neighbor chunks to be initialized");

        let chunk_data: [&ChunkData; 27] = chunk_data
            .try_into()
            .expect("number to elements to be exactly 27");

        Ok(Self { chunk_data })
    }

    pub fn get(&self, pos: ChunkLocation) -> &ChunkData {
        assert!(pos.x != 0 || pos.y != 0 || pos.z != 0);

        let idx = (pos.x + 1) * 9 + (pos.y + 1) * 3 + (pos.z + 1);
        return self.chunk_data[idx as usize];
    }
}

lazy_static! {
    static ref VOXEL_TYPE_RAND_MAP: EnumMap<VoxelType, Vec<Vector3<f32>>> = enum_map::enum_map! {
        VoxelType::Air => generate_voxel_type_map(VoxelType::Air),
        VoxelType::Dirt => generate_voxel_type_map(VoxelType::Dirt),
        VoxelType::Grass => generate_voxel_type_map(VoxelType::Grass),
        VoxelType::Stone => generate_voxel_type_map(VoxelType::Stone),
    };
}

fn generate_voxel_type_map(voxel_type: VoxelType) -> Vec<Vector3<f32>> {
    iproduct!(0..(CHUNK_SIZE as i32), 0..(CHUNK_SIZE as i32), 0..(CHUNK_SIZE as i32))
        .map(|(x, y, z)| {
            voxel_type_to_color(
                voxel_type,
                WorldLocation::new(
                    ChunkLocation::new(Vector3::new(0, 0, 0)),
                    LocalChunkLocation::new(Vector3::new(x, y, z)),
                ),
            )
        })
        .collect()
}

fn voxel_type_to_color_lookup(ty: VoxelType, local_voxel_position: &LocalChunkLocation<WithinBounds>) -> Vector3<f32> {
    VOXEL_TYPE_RAND_MAP[ty]
        .get(
            local_voxel_position.x as usize * CHUNK_SIZE * CHUNK_SIZE
                + local_voxel_position.y as usize * CHUNK_SIZE
                + local_voxel_position.z as usize,
        )
        .unwrap()
        .clone()
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
