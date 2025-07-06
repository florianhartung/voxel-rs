use cgmath::num_traits::Pow;
use cgmath::num_traits::real::Real;
use noise::{NoiseFn, Perlin};

use crate::world::CHUNK_SIZE;
use crate::world::chunk_data::ChunkData;
use crate::world::location::{ChunkLocation, LocalChunkLocation};
use crate::world::voxel_data::{VoxelData, VoxelType};

pub struct WorldGenerator {
    world_seed: u32,
}

impl WorldGenerator {
    pub fn new(world_seed: u32) -> Self {
        Self { world_seed }
    }

    pub fn get_chunk_data_at(&self, chunk_location: ChunkLocation) -> ChunkData {
        // ChunkData::new_with_uniform_data(VoxelData::world(VoxelType::Dirt))
        // flat_perlin_terrain(self.world_seed, chunk_location)
        perlin_3d(1, chunk_location)
        // ChunkData::Voxels(Box::new(CONST_CHUNK.clone()))
    }
}

const CONST_CHUNK: [VoxelData; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] = a();

const fn a() -> [VoxelData; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
    let mut data = [VoxelData::new(VoxelType::Air); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];

    // let mut x: usize = 0;
    // let mut y: usize = 0;
    // let mut z: usize = 0;

    // while x < CHUNK_SIZE {
    //     while z < CHUNK_SIZE {
    //         let height = (x + z) / 2;
    //         while y < CHUNK_SIZE {
    //             if y < height {
    //                 let index = z * CHUNK_SIZE.pow(2) + y * CHUNK_SIZE + x;
    //                 data[index] = VoxelData::new(VoxelType::Grass);
    //             }
    //             y += 1;
    //         }
    //
    //         z += 1;
    //     }
    //     x += 1;
    // }

    data[0] = VoxelData::new(VoxelType::Dirt);
    data[CHUNK_SIZE] = VoxelData::new(VoxelType::Stone);
    data[CHUNK_SIZE * CHUNK_SIZE] = VoxelData::new(VoxelType::Grass);

    data
}

pub fn waves(chunk_location: ChunkLocation) -> ChunkData {
    let mut chunk_voxel_data = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air));

    LocalChunkLocation::iter().for_each(|loc| {
        let coords = loc.to_f64() + chunk_location.to_world_location_f64();

        let height = 5.0 * (coords.x / 10.0).sin() + 5.0 * (coords.z / 10.0).sin();
        if coords.y < height {
            chunk_voxel_data.set_voxel_data(loc, VoxelData::new(VoxelType::Grass));
        }
    });

    chunk_voxel_data.try_convert_into_uniform();

    chunk_voxel_data
}

pub fn perlin_3d(world_seed: u32, chunk_location: ChunkLocation) -> ChunkData {
    let mut chunk_voxel_data = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air));
    let perlin = Perlin::new(world_seed);
    let perlin2 = Perlin::new(world_seed + 1);

    LocalChunkLocation::iter().for_each(|pos| {
        let coords = pos.to_f64() + chunk_location.to_world_location_f64();

        let density = perlin.get((coords * 0.01).into());

        if density < -0.2 {
            let ty_threshold = (perlin2.get((coords * 0.001).into()) + 1.0) / 2.0;
            let ty_threshold = ty_threshold.pow(5);
            let ty_rand = fastrand::f64();

            let ty = if ty_rand < ty_threshold {
                VoxelType::Stone
            } else {
                VoxelType::Grass
            };

            chunk_voxel_data.set_voxel_data(pos, VoxelData::new(ty));
        }
    });

    chunk_voxel_data
}
const EMPTY_CHUNK: ChunkData = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air));
const STONE_CHUNK: ChunkData = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Stone));

pub fn flat_perlin_terrain(world_seed: u32, chunk_location: ChunkLocation) -> ChunkData {
    // Create empty chunk data
    let mut chunk_voxel_data = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air));

    let mut perlin = Perlin::new(world_seed);
    let mut cave_perlin = Perlin::new(world_seed + 1);

    let octaves = vec![
        NoiseLayer { scale: 0.002, weight: 1.5 },
        NoiseLayer { scale: 0.007, weight: 0.9 },
        NoiseLayer { scale: 0.02, weight: 0.3 },
        NoiseLayer { scale: 0.07, weight: 0.06 },
        NoiseLayer { scale: 0.4, weight: 0.03 },
    ];

    if chunk_location.y > 2 {
        return EMPTY_CHUNK.clone();
    }

    if chunk_location.y < -3 {
        return STONE_CHUNK.clone();
    }

    // Fill empty chunk data with randomly selected voxels
    LocalChunkLocation::iter().for_each(|pos| {
        let coords = pos.to_f64() + chunk_location.to_world_location_f64();

        let layered_perlin = perlin.get_layered(&octaves, [coords.x, coords.z]);
        let normalized_height = (layered_perlin + 1.0) / 2.0;
        let height = 16.0 * normalized_height + 1.0;

        let voxel_type = if coords.y < height {
            {
                if coords.y + 1.0 < height {
                    if coords.y + 6.0 < height {
                        VoxelType::Stone
                    } else {
                        VoxelType::Dirt
                    }
                } else {
                    VoxelType::Grass
                }
            }
        } else {
            VoxelType::Air
        };

        if coords.y
            < cave_perlin.get_layered(
                &[
                    NoiseLayer { scale: 0.002, weight: 4.0 },
                    NoiseLayer { scale: 0.02, weight: 1.0 },
                    NoiseLayer { scale: 0.08, weight: 3.0 },
                ],
                [coords.x, coords.z],
            ) - 15.0
            && coords.y
                > cave_perlin.get_layered(
                    &[
                        NoiseLayer { scale: 0.002, weight: 3.0 },
                        NoiseLayer { scale: 0.04, weight: 3.0 },
                        NoiseLayer { scale: 0.08, weight: 0.3 },
                    ],
                    [coords.x, coords.z],
                ) - 30.0
            && cave_perlin.get_layered(
                &[
                    NoiseLayer { scale: 0.03, weight: 0.7 },
                    NoiseLayer { scale: 0.08, weight: 0.2 },
                    NoiseLayer { scale: 0.1, weight: 0.02 },
                ],
                [coords.x, coords.z],
            ) < 0.4 * cave_perlin.get([coords.y * 0.09, 0.0])
            || cave_perlin.get_layered(
                &[
                    NoiseLayer { scale: 0.03, weight: 0.7 },
                    NoiseLayer { scale: 0.08, weight: 0.2 },
                    NoiseLayer { scale: 0.1, weight: 0.02 },
                ],
                [coords.x, coords.z],
            ) < -0.8 + 0.5 * cave_perlin.get([coords.y * 0.02, coords.x * 0.02 + coords.z * 0.03])
                && coords.y > -30.0
        {
            // Air
        } else {
            chunk_voxel_data.set_voxel_data(pos, VoxelData::new(voxel_type));
        }
    });

    chunk_voxel_data.try_convert_into_uniform();

    chunk_voxel_data
}

struct NoiseLayer {
    pub weight: f64,
    pub scale: f64,
}

trait LayeredNoiseGenerator {
    fn get_layered(&mut self, octaves: &[NoiseLayer], point: [f64; 2]) -> f64;
}

impl LayeredNoiseGenerator for Perlin {
    fn get_layered(&mut self, octaves: &[NoiseLayer], point: [f64; 2]) -> f64 {
        octaves
            .iter()
            .map(|layer| layer.weight * self.get([point[0] * layer.scale, point[1] * layer.scale]))
            .sum()
    }
}
