use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cgmath::Vector3;
use strum::IntoEnumIterator;

use crate::engine::rendering::RenderCtx;
use crate::engine::vector_utils::{AbsValue, RemEuclid};
use crate::engine::world::chunk::Chunk;
use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation};
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
    pub fn generate_mesh(
        render_ctx: Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        location: ChunkLocation,
        chunks: &HashMap<ChunkLocation, Chunk>,
    ) -> Mesh {
        let quads = Self::generate_culled_mesh(
            location,
            &chunks
                .get(&location)
                .expect("Can't generate a mesh for a that does not exist")
                .data,
            chunks,
        );

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        quads.iter().for_each(|quad| {
            let base_index = vertices.len() as u32;

            let mut pos = quad.position.to_f32() + location.to_world_location_f32();
            let direction = quad
                .direction
                .to_vec()
                .cast::<f32>()
                .expect("Conversion from i32 to f32 is safe")
                .abs();

            let axis1 = Vector3::new(direction.y, direction.z, direction.x);
            let axis2 = Vector3::new(direction.z, direction.x, direction.y);

            let is_backside = match quad.direction {
                Direction::XPos | Direction::YPos | Direction::ZPos => false,
                Direction::XNeg | Direction::YNeg | Direction::ZNeg => true,
            };

            if !is_backside {
                pos += direction;
            }

            vertices.push(Vertex::new(pos, quad.data.color, direction));
            vertices.push(Vertex::new(pos + axis1, quad.data.color, direction));
            vertices.push(Vertex::new(pos + axis2, quad.data.color, direction));
            vertices.push(Vertex::new(pos + axis1 + axis2, quad.data.color, direction));

            {
                if is_backside {
                    [0, 1, 2, 2, 1, 3]
                } else {
                    [2, 1, 0, 3, 1, 2]
                }
            }
            .iter()
            .for_each(|i| indices.push(base_index + i));
        });

        Mesh::new(render_ctx, camera_bind_group_layout, vertices, indices)
    }

    pub fn generate_culled_mesh(
        current_location: ChunkLocation,
        data: &ChunkData,
        all_chunks: &HashMap<ChunkLocation, Chunk>,
    ) -> Vec<Quad> {
        let mut quads = Vec::new();

        LocalChunkLocation::iter()
            .filter(|&pos| data.get_voxel(pos).ty != VoxelType::Air)
            .for_each(|pos| {
                for dir in Direction::iter() {
                    let neighbor_voxel_location = pos + dir;

                    if let Some(same_chunk_neighbor) = neighbor_voxel_location.try_into_checked() {
                        if data.get_voxel(same_chunk_neighbor).ty == VoxelType::Air {
                            quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                        }
                    } else {
                        if let Some(chunk) = all_chunks.get(&ChunkLocation::new(*current_location + dir.to_vec())) {
                            let neighbor_local = LocalChunkLocation::new(neighbor_voxel_location.rem_euclid(CHUNK_SIZE as i32))
                                .try_into_checked()
                                .expect("aa");

                            if chunk.data.get_voxel(neighbor_local).ty == VoxelType::Air {
                                quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                            }
                        } else {
                            println!("wtf");
                        }
                    }

                    // if let Some(chunk) = all_chunks.get(&neighbor_chunk_location) {
                    //     if chunk.data.get_voxel(neighbor_voxel_location).ty == VoxelType::Air {
                    //         quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                    //     }
                    // } else {
                    //     quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                    // }

                    // if let Some(neighbor_location) = neighbor_voxel {
                    //     if data.get_voxel(neighbor_location).ty == VoxelType::Air {
                    //         quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                    //     }
                    // } else {
                    //     let (neighbor_chunk_location, local_pos) = WorldLocation::new(current_location, pos + dir).separate();
                    //
                    //     if let Some(chunk) = all_chunks.get(&neighbor_chunk_location) {
                    //         // in another chunk
                    //         if chunk.data.get_voxel(local_pos).ty == VoxelType::Air {
                    //             quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                    //         }
                    //     }
                    // }
                }
            });

        quads
    }
}

fn voxel_type_to_color(ty: VoxelType) -> Vector3<f32> {
    match ty {
        VoxelType::Air => Vector3::new(1.0, 0.0, 1.0),
        VoxelType::Dirt => Vector3::new(0.5, 0.5, 0.0),
        VoxelType::Grass => Vector3::new(0.1, 0.5, 0.0),
        VoxelType::Stone => Vector3::new(0.3, 0.3, 0.3),
    }
}
