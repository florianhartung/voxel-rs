use cgmath::num_traits::Pow;
use noise::{NoiseFn, Perlin};

use crate::engine::world::chunk_data::ChunkData;
use crate::engine::world::location::{ChunkLocation, LocalChunkLocation};
use crate::engine::world::voxel_data::{VoxelData, VoxelType};

pub struct WorldGenerator {
    _world_seed: u32,
}

impl WorldGenerator {
    pub fn new(world_seed: u32) -> Self {
        Self { _world_seed: world_seed }
    }

    pub fn get_chunk_data_at(&self, chunk_location: ChunkLocation) -> ChunkData {
        // ChunkData::new_with_uniform_data(VoxelData::world(VoxelType::Dirt))
        flat_perlin_terrain(1, chunk_location)
        // perlin_3d(1, chunk_location)
    }
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
