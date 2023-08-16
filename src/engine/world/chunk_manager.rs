use std::cell::RefCell;
use std::collections::vec_deque::VecDeque;
use std::mem;
use std::rc::Rc;

use cgmath::Vector3;
use itertools::{iproduct, Itertools};
use rayon::prelude::*;
use wgpu::{BindGroup, RenderPass};

use crate::engine::rendering::{RenderCtx, Renderer};
use crate::engine::timing::TimerManager;
use crate::engine::world::chunk::ChunkMesh;
use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::ChunkLocation;
use crate::engine::world::meshing::ChunkMeshGenerator;
use crate::engine::world::voxel_data::VoxelData;
use crate::engine::world::worldgen::WorldGenerator;
use crate::engine::world::CHUNK_SIZE;

pub struct ChunkManager {
    pub chunks: hashbrown::HashMap<ChunkLocation, ChunkData>,
    pub chunk_meshes: hashbrown::HashMap<ChunkLocation, ChunkMesh>,
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
            chunks: hashbrown::HashMap::new(),
            chunk_meshes: hashbrown::HashMap::new(),
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

    pub fn generate_chunks(&mut self, timer: &mut TimerManager) {
        timer.start("chunk_manager_generate_chunks");
        let load_distance = self.render_distance + 1;
        let last_player_position = self.last_player_position;

        timer.start("chunk_manager_fill_queue");
        if self.chunk_generate_queue.is_empty() && self.current_chunk_generate_radius < load_distance {
            self.current_chunk_generate_radius += 1;

            let radius = self.current_chunk_generate_radius;

            iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
                .map(|(x, y, z)| last_player_position + ChunkLocation::new(Vector3::new(x, y, z)))
                .for_each(|location| {
                    if !self.chunks.contains_key(&location) && !self.chunk_generate_queue.contains(&location) {
                        self.chunk_generate_queue.push_back(location);
                    }
                });
        }
        timer.end("chunk_manager_fill_queue");

        timer.start("chunk_manager_generation");
        let generated_chunks = self
            .chunk_generate_queue
            .drain(0..(8.min(self.chunk_generate_queue.len())))
            .par_bridge()
            .map(|location| (location, self.chunk_generator.get_chunk_data_at(location)))
            .collect::<Vec<_>>();
        timer.end("chunk_manager_generation");

        timer.start("chunk_manager_save");
        generated_chunks
            .into_iter()
            .for_each(|(location, data)| {
                match &data {
                    ChunkData::Voxels(_) => {
                        self.total_voxel_data_size += CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
                    }
                    ChunkData::UniformType(_) => {
                        self.total_voxel_data_size += mem::size_of::<VoxelData>();
                    }
                }

                self.chunks.insert(location, data);
            });
        timer.end("chunk_manager_save");

        timer.end("chunk_manager_generate_chunks");
    }

    pub fn generate_chunk_meshes(
        &mut self,
        render_ctx: &Rc<RefCell<RenderCtx>>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        timer: &mut TimerManager,
    ) {
        timer.start("chunk_manager_meshing");
        timer.start("chunk_manager_meshing_fill_queue");
        if self.chunk_mesh_queue.is_empty() && self.current_chunk_mesh_radius + 3 < self.current_chunk_generate_radius {
            self.current_chunk_mesh_radius += 1;

            let radius = self.current_chunk_mesh_radius;

            iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
                .map(|(x, y, z)| self.last_player_position + ChunkLocation::new(Vector3::new(x, y, z)))
                .for_each(|location| {
                    if self.chunks.contains_key(&location)
                        && !self.chunk_meshes.contains_key(&location)
                        && !self.chunk_mesh_queue.contains(&location)
                    {
                        self.chunk_mesh_queue.push_back(location);
                    }
                });
        }
        timer.end("chunk_manager_meshing_fill_queue");

        timer.start("chunk_manager_meshing_generate_meshes");

        let generated_meshes = self
            .chunk_mesh_queue
            .drain(0..(8.min(self.chunk_mesh_queue.len())))
            .par_bridge()
            .map(|location| {
                let data = self
                    .chunks
                    .get(&location)
                    .expect("Tried to generate mesh for chunk without data");
                (location, data)
            })
            .map(|(location, data)| {
                let quads = ChunkMeshGenerator::generate_culled_mesh(location, data, &self.chunks);

                (location, quads)
            })
            .collect::<Vec<_>>();
        timer.end("chunk_manager_meshing_generate_meshes");

        timer.start("chunk_manager_meshing_save");
        generated_meshes
            .into_iter()
            .for_each(|(location, quads)| {
                let mesh = ChunkMeshGenerator::generate_mesh_from_quads(location, quads, render_ctx.clone(), camera_bind_group_layout);
                self.total_vertices += mesh.vertices.len();
                self.total_triangles += mesh.indices.len() / 3;
                self.total_mesh_data_size += mem::size_of_val(mesh.indices.as_slice()) + mem::size_of_val(mesh.vertices.as_slice());

                self.chunk_meshes
                    .insert(location, ChunkMesh::new(mesh));
            });
        timer.end("chunk_manager_meshing_save");

        timer.end("chunk_manager_meshing");
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
                let chunk_data = self.chunks.remove(&loc).expect("wtf");
                self.chunk_mesh_queue.clear();
                self.chunk_generate_queue.retain(|l| l != &loc);

                if let Some(ChunkMesh::Generated(mesh)) = self.chunk_meshes.remove(&loc) {
                    self.total_vertices -= mesh.vertices.len();
                    self.total_triangles -= mesh.indices.len() / 3;
                    self.total_mesh_data_size -= mem::size_of_val(mesh.indices.as_slice()) + mem::size_of_val(mesh.vertices.as_slice());
                }

                match &chunk_data {
                    ChunkData::Voxels(_) => {
                        self.total_voxel_data_size -= CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
                    }
                    ChunkData::UniformType(_) => {
                        self.total_voxel_data_size -= mem::size_of::<VoxelData>();
                    }
                }
            }
        }
    }
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        self.chunk_meshes
            .iter()
            .for_each(|(_, chunk_mesh)| {
                if let Some(renderer) = chunk_mesh.get_renderer(self.render_empty_chunks) {
                    renderer.render(render_pass, camera_bind_group);
                }
            })
    }
}
