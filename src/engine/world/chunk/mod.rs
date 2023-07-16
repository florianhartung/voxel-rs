use std::ops::{Add, Deref, DerefMut};

use cgmath::Vector3;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::engine::rendering::RenderCtx;
use crate::engine::world::chunk::local_coordinates::LocalCoordinates;
use crate::engine::world::chunk::renderer::ChunkRenderer;
use crate::engine::world::mesh::Mesh;
use crate::engine::world::voxel::Voxel;

pub mod local_coordinates;
pub mod meshing;
pub mod renderer;

pub struct ChunkData {
    pub(crate) data: Box<[Voxel; Chunk::SIZE3 as usize]>,
}

impl ChunkData {
    pub fn new_with_uniform_type(voxel: Voxel) -> Self {
        ChunkData {
            data: Box::new([voxel; Chunk::SIZE3 as usize]),
        }
    }

    pub fn get_voxel_mut(&mut self, position: &LocalCoordinates) -> &mut Voxel {
        assert!(
            !position.is_out_of_bounds(),
            "Given local position is out of bounds"
        );

        &mut self.data[position.to_index(Chunk::SIZE) as usize]
    }

    pub fn get_voxel(&self, position: &LocalCoordinates) -> &Voxel {
        assert!(
            !position.is_out_of_bounds(),
            "Given local position is out of bounds"
        );

        &self.data[position.to_index(Chunk::SIZE) as usize]
    }

    pub fn get_neighboring_voxel(
        &self,
        pos: &LocalCoordinates,
        direction: &Direction,
    ) -> Option<&Voxel> {
        let neighbor_position: LocalCoordinates = pos.add(direction.to_vec()).into();
        if neighbor_position.is_out_of_bounds() {
            None
        } else {
            Some(self.get_voxel(&neighbor_position))
        }
    }
}

pub struct Chunk {
    data: ChunkData,
}

impl Chunk {
    const SIZE: u8 = 32;
    const SIZE2: u16 = (Self::SIZE as u16).pow(2);
    const SIZE3: u32 = (Self::SIZE as u32).pow(3);

    pub fn from_data(chunk_data: ChunkData) -> Self {
        Self { data: chunk_data }
    }

    pub fn into_meshed(self) -> MeshedChunk {
        let mesh = meshing::generate_mesh_from_chunk_data(&self.data);

        MeshedChunk {
            data: self.data,
            mesh,
        }
    }
}

pub struct MeshedChunk {
    data: ChunkData,
    mesh: Mesh,
}

impl MeshedChunk {
    pub fn get_renderer(
        &self,
        render_ctx: &RenderCtx,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> ChunkRenderer {
        ChunkRenderer {
            mesh_renderer: self.mesh.get_renderer(render_ctx, camera_bind_group_layout),
        }
    }

    pub fn randomize_data(&mut self) {
        LocalCoordinates::iter().for_each(|pos| {
            self.data.get_voxel_mut(&pos).ty = if fastrand::f32() < 0.5 { 1 } else { 0 }
        });

        self.mesh = meshing::generate_mesh_from_chunk_data(&self.data);
    }

    pub fn update_renderer(&self, chunk_renderer: &mut ChunkRenderer, render_ctx: &RenderCtx) {
        self.mesh
            .update_renderer(&mut chunk_renderer.mesh_renderer, render_ctx);
    }
}

#[derive(EnumIter)]
pub enum Direction {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}
impl Direction {
    pub fn to_vec(&self) -> Vector3<i16> {
        match self {
            Direction::XPos => Vector3::unit_x(),
            Direction::XNeg => -Vector3::unit_x(),
            Direction::YPos => Vector3::unit_y(),
            Direction::YNeg => -Vector3::unit_y(),
            Direction::ZPos => Vector3::unit_z(),
            Direction::ZNeg => -Vector3::unit_z(),
        }
    }
}
