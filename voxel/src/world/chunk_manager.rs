use std::collections::vec_deque::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{iter, mem, thread};

use anyhow::{Result, bail};
use cgmath::Vector3;
use crossbeam_queue::SegQueue;
use hashbrown::HashMap;
use itertools::{Itertools, iproduct};
use threadpool::ThreadPool;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BufferUsages, RenderPass, ShaderStages};

use crate::renderer::{RenderCtx, Renderer};
use crate::timing::TimerManager;
use crate::world::CHUNK_SIZE;
use crate::world::chunk_data::ChunkData;
use crate::world::chunk_renderer::ChunkRenderPipeline;
use crate::world::chunk_renderer::meshing::NeighborChunks;
use crate::world::location::ChunkLocation;
use crate::world::voxel_data::VoxelData;
use crate::world::worldgen::WorldGenerator;

use super::chunk_renderer::ChunkRenderer;
use super::chunk_renderer::meshing::ChunkMeshGenerator;
use super::chunk_renderer::vertex::{Instance, Vertex};

// maybe store nothing and use vertex_index inside shader
const VERTICES: [Vertex; 4] = [
    Vertex { xyz: 0 },
    Vertex { xyz: 1 },
    Vertex { xyz: 2 },
    Vertex { xyz: 3 },
    // Vertex { xyz: [0.0, 0.0, 0.0] },
    // Vertex { xyz: [1.0, 0.0, 0.0] },
    // Vertex { xyz: [0.0, 0.0, 1.0] },
    // Vertex { xyz: [1.0, 0.0, 1.0] },
];
const INDICES: [u32; 6] = [2, 3, 0, 0, 3, 1];

#[derive(Debug)]
pub enum Chunk {
    None {
        num_neighbors_generated: u8,
        queued_for_datagen: bool,
    },
    Generated {
        data: Arc<ChunkData>,
        num_neighbors_generated: u8,
        queued_for_meshing: bool,
    },
    Meshed {
        data: Arc<ChunkData>,
        renderer: Arc<ChunkRenderer>,
    },
}

impl Chunk {
    pub fn new() -> Self {
        Self::None {
            queued_for_datagen: false,
            num_neighbors_generated: 0,
        }
    }

    pub fn get_data(&self) -> Option<&Arc<ChunkData>> {
        match self {
            Chunk::Generated { data, .. } => Some(data),
            Chunk::Meshed { data, .. } => Some(data),
            Chunk::None { .. } => None,
        }
    }

    pub fn neighbor_count(&self) -> Option<u8> {
        match self {
            Chunk::None {
                num_neighbors_generated, ..
            } => Some(*num_neighbors_generated),
            Chunk::Generated {
                num_neighbors_generated, ..
            } => Some(*num_neighbors_generated),
            Chunk::Meshed { .. } => None,
        }
    }
    pub fn inc_neighbor_count(&mut self) -> Result<u8> {
        Ok(match self {
            Chunk::None {
                num_neighbors_generated, ..
            } => {
                *num_neighbors_generated += 1;
                *num_neighbors_generated
            }
            Chunk::Generated {
                num_neighbors_generated,
                queued_for_meshing: queued,
                ..
            } => {
                *num_neighbors_generated += 1;
                assert!(
                    !*queued,
                    "This chunk is already queued for mesh generation even though its neighbor count is {queued}"
                );
                *num_neighbors_generated
            }
            Chunk::Meshed { .. } => bail!("Cannot increase neighbor count of meshed chunk"),
        })
    }

    /// Tries to mark this chunk as queued for data generation. If it is already queued, it returns false
    pub fn enqueue_for_data_gen(&mut self) -> Result<bool> {
        if let Chunk::None {
            queued_for_datagen: queued,
            ..
        } = self
        {
            if !*queued {
                *queued = true;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            bail!("can not enqueue chunk for data generation: self={:?}", self)
        }
    }

    /// Tries to mark this chunk as queued for mesh generation. If it is already queued, it returns false
    pub fn enqueue_for_mesh_gen(&mut self) -> Result<bool> {
        if let Chunk::Generated {
            queued_for_meshing: queued,
            ..
        } = self
        {
            if !*queued {
                *queued = true;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            bail!("can not enqueue chunk for mesh generation: self={:?}", self)
        }
    }

    pub fn attach_data(&mut self, data: ChunkData) -> Result<()> {
        let Chunk::None {
            num_neighbors_generated, ..
        } = *self
        else {
            bail!(
                "Cannot attach data to a chunk that is not of the StoredChunk::None type. Note: Updating chunk data is not yet supported. self={:?}",
                self
            )
        };

        *self = Chunk::Generated {
            data: Arc::new(data),
            num_neighbors_generated,
            queued_for_meshing: false,
        };

        Ok(())
    }

    pub fn attach_mesh(&mut self, chunk_renderer: Arc<ChunkRenderer>) -> Result<()> {
        let Chunk::Generated { data, .. } = self else {
            bail!(
                "Cannot attach data to a chunk that is not of the StoredChunk::None type. self={:?}",
                self
            )
        };

        let data = data.clone();
        *self = Chunk::Meshed {
            data,
            renderer: chunk_renderer,
        };

        Ok(())
    }
}

pub struct ChunkManager {
    pub chunks: hashbrown::HashMap<ChunkLocation, Chunk>,
    pub renderers: hashbrown::HashMap<ChunkLocation, Arc<ChunkRenderer>>,

    chunk_generator: Arc<WorldGenerator>,
    last_player_position: ChunkLocation,
    current_chunk_generate_radius: i32,
    pub current_chunk_mesh_radius: i32,

    pub total_vertices: usize,
    pub total_triangles: usize,
    pub total_voxel_data_size: usize,
    pub total_mesh_data_size: usize,

    pub render_distance: i32,
    pub render_empty_chunks: bool,

    pub generated_chunks_queue: Arc<SegQueue<ChunkGenResult>>,
    pub meshed_chunks_queue: Arc<SegQueue<ChunkMeshResult>>,
    chunk_render_manager: ChunkRenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    pub worker_thread_pool: ThreadPool,
}

struct MeshGenQuery {
    chunk_data: [ChunkData; 27],
}

pub struct ChunkGenResult(ChunkLocation, ChunkData);

impl PartialEq for ChunkGenResult {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Hash for ChunkGenResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

pub struct ChunkMeshResult(ChunkLocation, Vec<Instance>);

const NUM_WORKERS: usize = 6;
const NUM_STREAM_MESHES_PER_FRAME: usize = 256;

impl ChunkManager {
    pub fn new(player_location: Vector3<f32>, render_ctx: &RenderCtx, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let chunk_generator = Arc::new(WorldGenerator::new(123));

        let generated_chunks_queue = Arc::new(SegQueue::new());
        let meshed_chunks_queue = Arc::new(SegQueue::new());

        let worker_thread_pool = ThreadPool::new(NUM_WORKERS);

        let vertex_buffer = render_ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunks vertex buffer"),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&VERTICES),
            });

        let index_buffer = render_ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunks index buffer"),
                usage: wgpu::BufferUsages::INDEX | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&INDICES),
            });

        Self {
            chunks: hashbrown::HashMap::new(),
            chunk_generator,
            last_player_position: ChunkLocation::from_world_location_f32(player_location),
            current_chunk_generate_radius: 0,
            current_chunk_mesh_radius: 0,
            total_vertices: 0,
            total_triangles: 0,
            total_voxel_data_size: 0,
            total_mesh_data_size: 0,
            render_distance: 16,
            render_empty_chunks: true,
            generated_chunks_queue,
            chunk_render_manager: ChunkRenderPipeline::new(&render_ctx, camera_bind_group_layout),
            worker_thread_pool,
            renderers: HashMap::new(),
            meshed_chunks_queue,
            vertex_buffer,
            index_buffer,
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

    pub fn enqueue_generation(&self, chunk_location: ChunkLocation) {
        let chunk_generator = Arc::clone(&self.chunk_generator);
        let generated_chunks_queue = Arc::clone(&self.generated_chunks_queue);

        self.worker_thread_pool.execute(move || {
            generated_chunks_queue.push(ChunkGenResult(chunk_location, chunk_generator.get_chunk_data_at(chunk_location)));
        });
    }

    pub fn generate_chunks(&mut self, timer: &mut TimerManager) {
        timer.start("chunk_manager_generate_chunks");
        let last_player_position = self.last_player_position;

        timer.start("chunk_manager_save");
        while let Some(ChunkGenResult(location, data)) = self.generated_chunks_queue.pop() {
            match &data {
                ChunkData::Voxels(_) => {
                    self.total_voxel_data_size += CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
                }
                ChunkData::UniformType(_) => {
                    self.total_voxel_data_size += mem::size_of::<VoxelData>();
                }
            }

            // let is_regeneration = match self.chunks.get(&location) {
            //     Some(Chunk::Generated {..}) => {
            //         TODO if is Generated {queued: true}, then we must remove this from the mesh queue
            // true
            // }
            // Some(Chunk::Meshed {..}) => {
            //     Todo regenerate mesh
            // true
            // }
            // _ => false,
            // };

            let chunk = self
                .chunks
                .entry(location)
                .or_insert_with(|| Chunk::new());

            chunk
                .attach_data(data)
                .expect("chunk data to not be present already");
            if chunk.neighbor_count() == Some(26) && chunk.enqueue_for_mesh_gen().unwrap() {
                self.enqueue_meshing(location);
            }

            // if !is_regeneration {
            iproduct!(-1..=1, -1..=1, -1..=1).for_each(|(dx, dy, dz)| {
                if dx == 0 && dy == 0 && dz == 0 {
                    return;
                }
                let loc = location + ChunkLocation::new(Vector3::new(dx, dy, dz));

                let chunk = self.chunks.entry(loc).or_insert(Chunk::new());
                let new_neighbor_count = chunk.inc_neighbor_count().expect(
                    "this chunk to not be meshed already, as the data for the current chunk (its neighbor chunk) has just been generated",
                );

                if new_neighbor_count == 26 {
                    match chunk.enqueue_for_mesh_gen() {
                        Ok(true) => self.enqueue_meshing(loc),
                        Err(_) => {
                            if chunk.enqueue_for_data_gen().unwrap() {
                                self.enqueue_generation(loc);
                            }
                        }
                        Ok(false) => {}
                    }
                }

                // #[cfg(debug_assertions)]
                // let chunks_unsafe = &self.chunks as *const hashbrown::HashMap<ChunkLocation, Chunk>;
                //
                // if let Some(s) = self.chunks.get_mut(&loc) {
                //     match s {
                //         Chunk::None { num_neighbors_generated, .. } => {
                //             *num_neighbors_generated += 1;
                //
                //             // Check if the num_neighbors_generated value is really correct.
                //             // This is implemented as a debug assertion as it may be costly when done for a lot of chunks
                //             #[cfg(debug_assertions)]
                //             {
                //
                //                 let mut count_generated = 0;
                //                 iproduct!(-1..=1, -1..=1, -1..=1).for_each(|(dx, dy, dz)| {
                //                     if dx == 0 && dy == 0 && dz == 0 {
                //                         return;
                //                     }
                //
                //                     let d = loc + ChunkLocation::new(Vector3::new(dx, dy, dz));
                //                     // # SAFETY
                //                     // self.chunks is currently mutably borrowed by this function.
                //                     // Thus no other thread has access to it and we can safely access it to look at values without storing them
                //                     let chunks_unsafe2 = unsafe { &*chunks_unsafe };
                //                     match chunks_unsafe2.get(&d) {
                //                         Some(Chunk::Generated { .. }) | Some(Chunk::Meshed { .. }) => {
                //                             count_generated += 1;
                //                         }
                //                         _ => {}
                //                     }
                //                 });
                //                 assert_eq!(count_generated, *num_neighbors_generated, "Invalid state of chunks where the None chunk at {loc:?} has a num_neighbors_generated of {}, but it really is {}", *num_neighbors_generated, count_generated);
                //             }
                //             if *num_neighbors_generated >= 27 {
                //                 panic!("a num_neighbors_generated of 27 should be impossible here, as there cannot exist an empty chunk with all of its neighbors chunks already generated");
                //             }
                //         }
                //         Chunk::Generated {
                //             num_neighbors_generated,
                //             queued,
                //             ..
                //         } => {
                //             assert_eq!(
                //                 *queued, false,
                //                 "chunk should not be queued for meshing already because this chunk's data has just been generated"
                //             );
                //
                //             *num_neighbors_generated += 1;
                //             if *num_neighbors_generated == 26 {
                //                 *queued = true;
                //                 self.chunk_mesh_queue.push_back(loc);
                //             }
                //         }
                //         Chunk::Meshed { .. } => {
                //             panic!("chunk should not be meshed already because this neighbor chunk's data has just been generated")
                //         }
                //     }
            });
            // }
        }
        timer.end("chunk_manager_save");

        timer.start("chunk_manager_request_chunks");
        if self.worker_thread_pool.queued_count() == 0 && self.current_chunk_generate_radius < self.render_distance {
            self.current_chunk_generate_radius += 1;

            let radius = self.current_chunk_generate_radius;

            iproduct!(-radius..=radius, -radius..=radius, -radius..=radius)
                .map(|(x, y, z)| last_player_position + ChunkLocation::new(Vector3::new(x, y, z)))
                .for_each(|location| {
                    let c = self
                        .chunks
                        .entry(location)
                        .or_insert(Chunk::new());

                    if let Ok(true) = c.enqueue_for_data_gen() {
                        self.enqueue_generation(location);
                    }
                });
        }
        timer.end("chunk_manager_request_chunks");

        timer.end("chunk_manager_generate_chunks");
    }

    pub fn stream_chunk_meshes(&mut self, ctx: &RenderCtx, camera_bind_group_layout: &wgpu::BindGroupLayout, timer: &mut TimerManager) {
        timer.start("chunk_manager_stream_meshes");

        for _ in 0..NUM_STREAM_MESHES_PER_FRAME {
            let Some(ChunkMeshResult(loc, instances)) = self.meshed_chunks_queue.pop() else {
                break;
            };

            let instance_buffer = (instances.len() > 0).then(|| {
                ctx.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("chunks instance buffer"),
                        usage: wgpu::BufferUsages::VERTEX | BufferUsages::COPY_DST,
                        contents: bytemuck::cast_slice(&instances),
                    })
            });

            self.total_mesh_data_size += VERTICES.len() * std::mem::size_of::<Vertex>()
                + INDICES.len() * std::mem::size_of::<u32>()
                + instances.len() * std::mem::size_of::<Instance>();

            let chunk_renderer = Arc::new(ChunkRenderer {
                instance_buffer,
                num_instances: instances.len() as u32,
            });

            self.chunks
                .get_mut(&loc)
                .expect("Tried to generate mesh for chunk without data")
                .attach_mesh(chunk_renderer.clone())
                .expect("this to not already have a mesh");

            self.renderers.insert(loc, chunk_renderer);
        }

        timer.end("chunk_manager_stream_meshes");
    }

    pub fn unload_chunks(&mut self) {
        return;
        //     let unload_distance = self.render_distance;
        //
        //     let a: Vec<ChunkLocation> = self.chunks.keys().copied().collect_vec();
        //     for loc in a {
        //         let location_relative_to_player = self.last_player_position - loc;
        //
        //         if !((-unload_distance..=unload_distance).contains(&location_relative_to_player.x)
        //             && (-unload_distance..=unload_distance).contains(&location_relative_to_player.y)
        //             && (-unload_distance..=unload_distance).contains(&location_relative_to_player.z))
        //         {
        //             let chunk_data = self.chunks.remove(&loc).expect("wtf");
        //             self.chunk_mesh_queue.clear();
        //             //self.chunk_generate_queue.retain(|l| l != &loc);
        //
        //             if let Some(ChunkMesh::Generated(mesh)) = self.chunk_meshes.remove(&loc) {
        //                 self.total_vertices -= mesh.vertices.len();
        //                 self.total_triangles -= mesh.indices.len() / 3;
        //                 self.total_mesh_data_size -= mem::size_of_val(mesh.indices.as_slice()) + mem::size_of_val(mesh.vertices.as_slice());
        //             }
        //
        //             match &chunk_data {
        //                 ChunkData::Voxels(_) => {
        //                     self.total_voxel_data_size -= CHUNK_SIZE.pow(3) * mem::size_of::<VoxelData>();
        //                 }
        //                 ChunkData::UniformType(_) => {
        //                     self.total_voxel_data_size -= mem::size_of::<VoxelData>();
        //                 }
        //             }
        //         }
        //     }
        // }
    }

    fn enqueue_meshing(&self, loc: ChunkLocation) {
        let neighbor_chunks = NeighborChunks::new(&loc, |loc| {
            self.chunks
                .get(loc)
                .map(|chunk| chunk.get_data().cloned())
                .flatten()
        })
        .unwrap();

        let chunk_data = self
            .chunks
            .get(&loc)
            .unwrap()
            .get_data()
            .unwrap()
            .clone();
        let meshed_chunks_queue = self.meshed_chunks_queue.clone();
        self.worker_thread_pool.execute(move || {
            let quads = ChunkMeshGenerator::generate_culled_mesh(&*chunk_data, neighbor_chunks);
            let instances = ChunkMeshGenerator::generate_mesh_from_quads(quads);

            meshed_chunks_queue.push(ChunkMeshResult(loc, instances));
        });
    }
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, mut render_pass: RenderPass<'a>, camera_bind_group: &'a BindGroup, _render_ctx: &RenderCtx) {
        for (position, renderer) in &self.renderers {
            if let Some(instance_buffer) = &renderer.instance_buffer {
                render_pass.set_pipeline(&self.chunk_render_manager.render_pipeline);

                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                render_pass.set_bind_group(0, camera_bind_group, &[]);

                // Push current chunk location
                let loc = [position.to_world_location_f32()];
                render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&loc));

                render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..renderer.num_instances);
            }
        }
    }
}
