use std::ops::{Add, Deref, DerefMut};

use cgmath::Vector3;
use itertools::iproduct;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::engine::rendering::RenderCtx;
use crate::engine::world::chunk::local_coordinates::LocalCoordinates;
use crate::engine::world::chunk::renderer::ChunkRenderer;
use crate::engine::world::mesh::{Mesh, MeshVertex};
use crate::engine::world::voxel::Voxel;

pub mod local_coordinates;
pub mod renderer;

pub struct ChunkData {
    data: Box<[Voxel; Chunk::SIZE3 as usize]>,
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

    pub fn new_with_random_data() -> Self {
        // Create empty chunk data
        let mut chunk_voxel_data = ChunkData::new_with_uniform_type(Voxel::default());

        // Fill empty chunk data with randomly selected voxels
        chunk_voxel_data
            .data
            .iter_mut()
            .enumerate()
            .for_each(|(_i, v)| v.ty = if fastrand::f32() < 0.1 { 1 } else { 0 });

        Self {
            data: chunk_voxel_data,
        }
    }

    pub fn into_meshed(self) -> MeshedChunk {
        let mesh = generate_mesh_from_chunk_data(&self.data);

        MeshedChunk {
            data: self.data,
            mesh,
        }
    }
}

fn generate_mesh_from_chunk_data(data: &ChunkData) -> Mesh {
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

    let mut add_tri =
        |pos: LocalCoordinates, a: (i16, i16, i16), b: (i16, i16, i16), c: (i16, i16, i16)| {
            indices.push(pos.add(Vector3::from(a)).to_index(vertices_per_direction));
            indices.push(pos.add(Vector3::from(b)).to_index(vertices_per_direction));
            indices.push(pos.add(Vector3::from(c)).to_index(vertices_per_direction));
        };

    LocalCoordinates::iter()
        .map(|pos| (pos, data.get_voxel(&pos)))
        .filter(|(_pos, voxel)| voxel.ty == 1)
        .for_each(|(pos, _voxel)| {
            for dir in Direction::iter() {
                match dir {
                    Direction::XPos => {
                        add_tri(pos, (1, 0, 0), (1, 1, 0), (1, 0, 1));
                        add_tri(pos, (1, 1, 1), (1, 1, 0), (1, 0, 1));
                    }
                    Direction::XNeg => {
                        add_tri(pos, (0, 0, 0), (0, 1, 0), (0, 0, 1));
                        add_tri(pos, (0, 1, 1), (0, 1, 0), (0, 0, 1));
                    }
                    Direction::YPos => {
                        add_tri(pos, (0, 1, 0), (1, 1, 0), (0, 1, 1));
                        add_tri(pos, (1, 1, 1), (1, 1, 0), (0, 1, 1));
                    }
                    Direction::YNeg => {
                        add_tri(pos, (0, 0, 0), (1, 0, 0), (0, 0, 1));
                        add_tri(pos, (1, 0, 1), (1, 0, 0), (0, 0, 1));
                    }
                    Direction::ZPos => {
                        add_tri(pos, (0, 0, 1), (1, 0, 1), (0, 1, 1));
                        add_tri(pos, (1, 1, 1), (1, 0, 1), (0, 1, 1));
                    }
                    Direction::ZNeg => {
                        add_tri(pos, (0, 0, 0), (1, 0, 0), (0, 1, 0));
                        add_tri(pos, (1, 1, 0), (1, 0, 0), (0, 1, 0));
                    }
                }
            }
        });

    Mesh::new(vertices, indices)
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

        self.mesh = generate_mesh_from_chunk_data(&self.data);
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
