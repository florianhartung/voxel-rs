use std::cell::RefCell;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

use cgmath::Vector3;
use itertools::{iproduct, Itertools};
use wgpu::{BindGroup, RenderPass};

use crate::engine::rendering::{RenderCtx, Renderer};
use crate::engine::world::chunk::Chunk;
use crate::engine::world::location::ChunkLocation;
use crate::engine::world::meshing::ChunkMeshGenerator;
use crate::engine::world::voxel_data::VoxelData;
use crate::engine::world::worldgen::WorldGenerator;
use crate::engine::world::CHUNK_SIZE;

pub struct ChunkManager {
    pub chunks: HashMap<ChunkLocation, Chunk>,
    chunk_generator: WorldGenerator,
    last_player_position: ChunkLocation,
    chunk_generate_queue: VecDeque<ChunkLocation>,
    chunk_mesh_queue: VecDeque<ChunkLocation>,
    current_chunk_generate_radius: i32,
    pub current_chunk_mesh_radius: i32,

    pub total_vertices: usize,
    pub total_triangles: usize,
    pub total_voxel_data_size: usize,
    pub total_mesh_data_size: usize,

    pub render_distance: i32,
    pub render_empty_chunks: bool,
}

impl ChunkManager {
    pub fn new(player_location: Vector3<f32>) -> Self {
        let chunk_generator = WorldGenerator::new(123);

        Self {
            chunks: HashMap::new(),
            chunk_generator,
            last_player_position: ChunkLocation::from_world_location_f32(player_location),
            chunk_generate_queue: VecDeque::new(),
            chunk_mesh_queue: VecDeque::new(),
            current_chunk_generate_radius: 0,
            current_chunk_mesh_radius: 0,
            total_vertices: 0,
            total_triangles: 0,
            total_voxel_data_size: 0,
            total_mesh_data_size: 0,
            render_distance: 16,
            render_empty_chunks: true,
        }
    }

    pub fn update_player_location(&mut self, player_location: Vector3<f32>) {
        let new_chunk_location = ChunkLocation::from_world_location_f32(player_location);
        if new_chunk_location != self.last_player_position {
            self.current_chunk_generate_radius = 0;
            self.current_chunk_mesh_radius = 0;
            self.last_player_position = ChunkLocation::from_world_location_f32(player_location);
        }
    }

    pub fn generate_chunks(&mut self) {
        let load_distance = self.render_distance + 1;
        let last_player_position = self.last_player_position;

        if self.chunk_generate_queue.is_empty() {
            if self.current_chunk_generate_radius < load_distance {
                self.current_chunk_generate_radius += 1;

                let radius = self.current_chunk_generate_radius;

                iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
                    .into_iter()
                    .map(|(x, y, z)| last_player_position + ChunkLocation::new(Vector3::new(x, y, z)))
                    .for_each(|location| {
                        if !self.chunks.contains_key(&location) {
                            if !self.chunk_generate_queue.contains(&location) {
                                self.chunk_generate_queue.push_back(location);
                            }
                        }
                    });
            }
        }

        let mut count = 0;
        while let Some(location) = self.chunk_generate_queue.pop_back() {
            self.generate_new(location);

            count += 1;
            if count >= 3 {
                break;
            }
        }
    }

    pub fn generate_chunk_meshes(&mut self, render_ctx: &Rc<RefCell<RenderCtx>>, camera_bind_group_layout: &wgpu::BindGroupLayout) {
        let mesh_distance = self.render_distance;

        if self.chunk_mesh_queue.is_empty() {
            if self.current_chunk_mesh_radius < mesh_distance && self.current_chunk_mesh_radius + 2 < self.current_chunk_generate_radius {
                self.current_chunk_mesh_radius += 1;

                let radius = self.current_chunk_mesh_radius;

                iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
                    .into_iter()
                    .map(|(x, y, z)| self.last_player_position + ChunkLocation::new(Vector3::new(x, y, z)))
                    .for_each(|location| {
                        if let Some(chunk) = self.chunks.get_mut(&location) {
                            if chunk.mesh.is_none() {
                                if !self.chunk_mesh_queue.contains(&chunk.location) {
                                    self.chunk_mesh_queue.push_back(chunk.location);
                                }
                            }
                        }
                    });
            }
        }

        let mut count = 0;
        while let Some(loc) = self.chunk_mesh_queue.pop_back() {
            let mesh = ChunkMeshGenerator::generate_mesh(render_ctx.clone(), camera_bind_group_layout, loc, &self.chunks);

            self.total_vertices += mesh.vertices.len();
            self.total_triangles += mesh.indices.len() / 3;
            self.total_mesh_data_size += mem::size_of_val(mesh.indices.as_slice()) + mem::size_of_val(&mesh.vertices.as_slice());

            self.chunks
                .get_mut(&loc)
                .expect("Can not insert mesh into a non-existing chunk")
                .mesh = Some(mesh);

            count += 1;
            if count >= 3 {
                break;
            }
        }
    }

    pub fn unload_chunks(&mut self) {
        let unload_distance = self.render_distance;

        let a: Vec<ChunkLocation> = self.chunks.keys().copied().collect_vec();
        for loc in a {
            let location_relative_to_player = self.last_player_position - loc;

            if !((-unload_distance..=unload_distance).contains(&location_relative_to_player.x)
                && (-unload_distance..=unload_distance).contains(&location_relative_to_player.y)
                && (-unload_distance..=unload_distance).contains(&location_relative_to_player.z))
            {
                let chunk = self.chunks.remove(&loc).expect("wtf");
                self.chunk_mesh_queue.retain(|l| l != &loc);
                self.chunk_generate_queue.retain(|l| l != &loc);

                if let Some(mesh) = chunk.mesh {
                    self.total_vertices -= mesh.vertices.len();
                    self.total_triangles -= mesh.indices.len() / 3;
                    self.total_mesh_data_size -= mem::size_of_val(mesh.indices.as_slice()) + mem::size_of_val(&mesh.vertices.as_slice());
                }

                self.total_voxel_data_size -= CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
            }
        }
    }

    fn generate_new(&mut self, location: ChunkLocation) {
        let chunk_data = self.chunk_generator.get_chunk_data_at(location);
        let chunk = Chunk::new(location, chunk_data);
        self.chunks.insert(location, chunk);
        self.total_voxel_data_size += CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
    }
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        self.chunks.iter().for_each(|(_, chunk)| {
            if let Some(renderer) = chunk.get_renderer(self.render_empty_chunks) {
                renderer.render(render_pass, camera_bind_group);
            }
        })
    }
}
