use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use cgmath::Vector3;
use itertools::iproduct;
use wgpu::{BindGroup, RenderPass};

use crate::engine::rendering::{RenderCtx, Renderer};
use crate::engine::world::chunk::Chunk;
use crate::engine::world::location::ChunkLocation;
use crate::engine::world::meshing::ChunkMeshGenerator;
use crate::engine::world::worldgen::WorldGenerator;

const RENDER_DISTANCE: i32 = 16;
const LOAD_DISTANCE: i32 = RENDER_DISTANCE + 2;

const UNLOAD_DISTANCE: i32 = RENDER_DISTANCE + 4;

pub struct ChunkManager {
    pub chunks: HashMap<ChunkLocation, Chunk>,
    chunk_generator: WorldGenerator,
    last_player_position: ChunkLocation,
    chunk_generate_queue: VecDeque<ChunkLocation>,
    chunk_mesh_queue: VecDeque<ChunkLocation>,
    current_chunk_generate_radius: i32,
    current_chunk_mesh_radius: i32,
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
        let last_player_position = self.last_player_position;

        if self.chunk_generate_queue.is_empty() {
            if self.current_chunk_generate_radius < LOAD_DISTANCE {
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
        if self.chunk_mesh_queue.is_empty() {
            if self.current_chunk_mesh_radius < RENDER_DISTANCE && self.current_chunk_mesh_radius + 1 < self.current_chunk_generate_radius {
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
        self.chunks.retain(|location, _| {
            let location_relative_to_player = self.last_player_position - *location;
            (-UNLOAD_DISTANCE..=UNLOAD_DISTANCE).contains(&location_relative_to_player.x)
                && (-UNLOAD_DISTANCE..=UNLOAD_DISTANCE).contains(&location_relative_to_player.y)
                && (-UNLOAD_DISTANCE..=UNLOAD_DISTANCE).contains(&location_relative_to_player.z)
        });
    }

    fn generate_new(&mut self, location: ChunkLocation) {
        let chunk_data = self.chunk_generator.get_chunk_data_at(location);
        let chunk = Chunk::new(location, chunk_data);
        self.chunks.insert(location, chunk);
    }
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        self.chunks.iter().for_each(|(_, chunk)| {
            if let Some(renderer) = chunk.get_renderer() {
                renderer.render(render_pass, camera_bind_group);
            }
        })
    }
}
