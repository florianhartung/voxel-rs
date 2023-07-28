use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cgmath::Vector3;
use strum::IntoEnumIterator;

use crate::engine::rendering::RenderCtx;
use crate::engine::vector_utils::AbsValue;
use crate::engine::world::chunk::Chunk;
use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation};
use crate::engine::world::mesh::{Mesh, Vertex};
use crate::engine::world::meshing::direction::Direction;
use crate::engine::world::meshing::quad::{FaceData, Quad};
use crate::engine::world::voxel_data::VoxelType;

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
            &chunks
                .get(&location)
                .expect("Can't generate a mesh for a that does not exist")
                .data,
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

            vertices.push(Vertex::new(pos, quad.data.color));
            vertices.push(Vertex::new(pos + axis1, quad.data.color));
            vertices.push(Vertex::new(pos + axis2, quad.data.color));
            vertices.push(Vertex::new(pos + axis1 + axis2, quad.data.color));

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

    pub fn generate_culled_mesh(data: &ChunkData) -> Vec<Quad> {
        let mut quads = Vec::new();

        LocalChunkLocation::iter()
            .filter(|&pos| data.get_voxel(pos).ty != VoxelType::Air)
            .for_each(|pos| {
                for dir in Direction::iter() {
                    let neighbor_voxel = (pos + dir).try_into_checked();

                    if neighbor_voxel
                        .filter(|&x| data.get_voxel(x).ty != VoxelType::Air)
                        .is_none()
                    {
                        quads.push(Quad::new(pos, dir, FaceData::new(voxel_type_to_color(data.get_voxel(pos).ty))));
                    }
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
