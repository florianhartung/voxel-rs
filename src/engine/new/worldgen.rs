use noise::{NoiseFn, Perlin};

use crate::engine::new::chunk_data::ChunkData;
use crate::engine::new::location::{ChunkLocation, LocalChunkLocation};
use crate::engine::new::voxel_data::{VoxelData, VoxelType};

pub struct WorldGenerator {
    world_seed: u32,
}

impl WorldGenerator {
    pub fn new(world_seed: u32) -> Self {
        Self { world_seed }
    }

    pub fn get_chunk_data_at(&self, chunk_location: ChunkLocation) -> ChunkData {
        // ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Dirt))
        generate_perlin_terrain(1, chunk_location)
    }
}

pub fn generate_perlin_terrain(world_seed: u32, chunk_location: ChunkLocation) -> ChunkData {
    // Create empty chunk data
    let mut chunk_voxel_data = ChunkData::new_with_uniform_data(VoxelData::new(VoxelType::Air));

    let mut perlin = Perlin::new(world_seed);

    let octaves = vec![
        NoiseLayer { scale: 0.002, weight: 1.5 },
        NoiseLayer { scale: 0.007, weight: 0.9 },
        NoiseLayer { scale: 0.02, weight: 0.3 },
        NoiseLayer { scale: 0.07, weight: 0.06 },
        NoiseLayer { scale: 0.4, weight: 0.03 },
    ];

    // Fill empty chunk data with randomly selected voxels
    LocalChunkLocation::iter().for_each(|pos| {
        let coords = pos.to_f64() + chunk_location.to_world_position_f64();

        let layered_perlin = perlin.get_layered(&octaves, [coords.x, coords.z]);
        let normalized_height = (layered_perlin + 1.0) / 2.0;
        let height = 16.0 * normalized_height + 1.0;

        chunk_voxel_data.get_voxel_mut(pos).ty = if coords.y < height {
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
    });

    chunk_voxel_data
}

struct NoiseLayer {
    pub weight: f64,
    pub scale: f64,
}

trait LayeredNoiseGenerator {
    fn get_layered(&mut self, octaves: &Vec<NoiseLayer>, point: [f64; 2]) -> f64;
}

impl LayeredNoiseGenerator for Perlin {
    fn get_layered(&mut self, octaves: &Vec<NoiseLayer>, point: [f64; 2]) -> f64 {
        octaves
            .iter()
            .map(|layer| layer.weight * self.get([point[0] * layer.scale, point[1] * layer.scale]))
            .sum()
    }
}
