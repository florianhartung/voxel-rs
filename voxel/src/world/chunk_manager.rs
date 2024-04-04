use std::collections::vec_deque::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{mem, thread};

use anyhow::{bail, Result};
use cgmath::Vector3;
use itertools::{iproduct, Itertools};
use rayon::prelude::*;
use wgpu::{BindGroup, RenderPass};

use crate::rendering::{RenderCtx, Renderer};
use crate::timing::TimerManager;
use crate::world::awesome_queue::AwesomeQueue;
use crate::world::chunk_data::ChunkData;
use crate::world::chunk_renderer::meshing::NeighborChunks;
use crate::world::chunk_renderer::ChunkRenderManager;
use crate::world::location::ChunkLocation;
use crate::world::voxel_data::{VoxelData, VoxelType};
use crate::world::worldgen::WorldGenerator;
use crate::world::CHUNK_SIZE;

#[derive(Debug)]
pub enum Chunk {
    None {
        num_neighbors_generated: u8,
        queued_for_datagen: bool,
    },
    Generated {
        data: ChunkData,
        num_neighbors_generated: u8,
        queued_for_meshing: bool,
    },
    Meshed {
        data: ChunkData,
    },
}

impl Chunk {
    pub fn new() -> Self {
        Self::None {
            queued_for_datagen: false,
            num_neighbors_generated: 0,
        }
    }

    pub fn get_data(&self) -> Option<&ChunkData> {
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
            data,
            num_neighbors_generated,
            queued_for_meshing: false,
        };

        Ok(())
    }

    pub fn attach_mesh(&mut self) -> Result<()> {
        let Chunk::Generated { data, .. } = self else {
            bail!(
                "Cannot attach data to a chunk that is not of the StoredChunk::None type. self={:?}",
                self
            )
        };

        /// Transform ownership of the chunk data from the enum variant [Chunk::Generated] to [Chunk::Meshed].
        /// To do this the ownership of the previous chunk data is taken by replacing it with a temporary value.
        const TEMP_EMPTY_DATA: ChunkData = ChunkData::UniformType(VoxelData::new(VoxelType::Air));
        let previous_chunk_data = mem::replace(data, TEMP_EMPTY_DATA.clone());

        *self = Chunk::Meshed { data: previous_chunk_data };

        Ok(())
    }
}

pub struct ChunkManager {
    pub chunks: hashbrown::HashMap<ChunkLocation, Chunk>,
    chunk_generator: Arc<WorldGenerator>,
    last_player_position: ChunkLocation,
    pub chunk_mesh_queue: VecDeque<ChunkLocation>,
    current_chunk_generate_radius: i32,
    pub current_chunk_mesh_radius: i32,

    pub total_vertices: usize,
    pub total_triangles: usize,
    pub total_voxel_data_size: usize,
    pub total_mesh_data_size: usize,

    pub render_distance: i32,
    pub render_empty_chunks: bool,

    pub location_queue: Arc<AwesomeQueue<ChunkLocation>>,
    pub generated_chunks_queue: Arc<AwesomeQueue<ChunkGenResult>>,
    // pub mesh_gen_queue: Arc<AwesomeQueue<(ChunkLocation)>>,
    // pub generated_meshes_queue: Arc<AwesomeQueue<ChunkGenResult>>,
    chunk_render_manager: ChunkRenderManager,
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

const NUM_DATA_GEN_THREAD: usize = 8;
const DATA_GEN_THREAD_BATCH_SIZE: usize = 20;

impl ChunkManager {
    pub fn new(player_location: Vector3<f32>, render_ctx: &RenderCtx, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let chunk_generator = Arc::new(WorldGenerator::new(123));

        let location_queue: Arc<AwesomeQueue<ChunkLocation>> = Arc::new(AwesomeQueue::new());
        let generated_chunks_queue: Arc<AwesomeQueue<ChunkGenResult>> = Arc::new(AwesomeQueue::new());

        for _ in 0..NUM_DATA_GEN_THREAD {
            let chunk_generator = Arc::clone(&chunk_generator);
            let location_queue = Arc::clone(&location_queue);
            let generated_chunks_queue = Arc::clone(&generated_chunks_queue);
            thread::Builder::new()
                .name("chunk data generator".to_owned())
                .spawn(move || loop {
                    let chunk_locs = location_queue.take_n(DATA_GEN_THREAD_BATCH_SIZE);

                    if chunk_locs.len() == 0 {
                        thread::sleep(Duration::from_millis(5));
                    }

                    chunk_locs
                        .into_iter()
                        .for_each(|loc| generated_chunks_queue.insert(ChunkGenResult(loc, chunk_generator.get_chunk_data_at(loc))));
                })
                .unwrap();
        }

        Self {
            chunks: hashbrown::HashMap::new(),
            chunk_generator,
            last_player_position: ChunkLocation::from_world_location_f32(player_location),
            chunk_mesh_queue: VecDeque::new(),
            current_chunk_generate_radius: 0,
            current_chunk_mesh_radius: 0,
            total_vertices: 0,
            total_triangles: 0,
            total_voxel_data_size: 0,
            total_mesh_data_size: 0,
            render_distance: 16,
            render_empty_chunks: true,
            location_queue,
            generated_chunks_queue,
            chunk_render_manager: ChunkRenderManager::new(&render_ctx, camera_bind_group_layout),
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
        let last_player_position = self.last_player_position;

        timer.start("chunk_manager_save");
        self.generated_chunks_queue
            .take_all()
            .into_iter()
            .for_each(|ChunkGenResult(location, data)| {
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

                let mut chunk = self.chunks.entry(location).or_insert_with(|| Chunk::new());
                chunk.attach_data(data).expect("chunk data to not be present already");
                if chunk.neighbor_count() == Some(26) && chunk.enqueue_for_mesh_gen().unwrap() {
                    self.chunk_mesh_queue.push_back(location);
                }


                // if !is_regeneration {
                iproduct!(-1..=1, -1..=1, -1..=1).for_each(|(dx, dy, dz)| {
                    if dx == 0 && dy == 0 && dz == 0 {
                        return;
                    }
                    let loc = location + ChunkLocation::new(Vector3::new(dx, dy, dz));

                    let chunk = self.chunks.entry(loc).or_insert(Chunk::new());
                    let new_neighbor_count = chunk.inc_neighbor_count().expect("this chunk to not be meshed already, as the data for the current chunk (its neighbor chunk) has just been generated");

                    if new_neighbor_count == 26 {
                        match chunk.enqueue_for_mesh_gen() {
                            Ok(true) => self.chunk_mesh_queue.push_back(loc),
                            Err(_) => if chunk.enqueue_for_data_gen().unwrap() { self.location_queue.insert(loc) },
                            Ok(false) => {},
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
                })
                // }
            });
        timer.end("chunk_manager_save");

        timer.start("chunk_manager_request_chunks");
        if self.location_queue.len() == 0 && self.current_chunk_generate_radius < self.render_distance && self.chunk_mesh_queue.len() < 500
        {
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
                        self.location_queue.insert(location);
                    }
                });
        }
        timer.end("chunk_manager_request_chunks");

        timer.end("chunk_manager_generate_chunks");
    }

    pub fn generate_chunk_meshes(
        &mut self,
        render_ctx: &RenderCtx,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        timer: &mut TimerManager,
    ) {
        const MAX_TIME: Duration = Duration::from_millis(2);

        let start = Instant::now();

        timer.start("chunk_manager_meshing");

        while start.elapsed() < MAX_TIME && self.chunk_mesh_queue.len() > 0 {
            let locs_to_be_meshed = self
                .chunk_mesh_queue
                .drain(0..(8.min(self.chunk_mesh_queue.len())))
                .collect_vec();

            locs_to_be_meshed
                .into_iter()
                // .filter(|location| {
                //     let stored_chunk = self.chunks.get(location);
                //     // TODO fix this check. currently an empty chunk will be 'queued' forever, even though it is not in the queue anymore
                //     !matches!(
                //         stored_chunk,
                //         Some(Chunk::Generated {
                //             data: ChunkData::UniformType(VoxelData { ty: VoxelType::Air }),
                //             ..
                //         })
                //     )
                // })
                .for_each(|location| {
                    let data = {
                        let stored_chunk = self
                            .chunks
                            .get_mut(&location)
                            .expect("Tried to generate mesh for chunk without data");

                        let Chunk::Generated {
                            data,
                            num_neighbors_generated: 26,
                            queued_for_meshing: true,
                        } = stored_chunk
                        else {
                            panic!("Found invalid chunk while trying to generate mesh");
                        };

                        data.clone()
                    };

                    let neighbor_chunks = NeighborChunks::new(&location, |loc| {
                        self.chunks
                            .get(loc)
                            .map(Chunk::get_data)
                            .flatten()
                    })
                    .unwrap();

                    self.chunk_render_manager
                        .generate_chunk_renderer(&data, neighbor_chunks, render_ctx, location);

                    self.chunks
                        .get_mut(&location)
                        .expect("Tried to generate mesh for chunk without data")
                        .attach_mesh()
                        .expect("this to not already have a mesh");
                });
        }

        timer.end("chunk_manager_meshing");
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
}

impl Renderer for ChunkManager {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup, render_ctx: &RenderCtx) {
        self.chunk_render_manager
            .render(render_pass, camera_bind_group, render_ctx);
    }
}
