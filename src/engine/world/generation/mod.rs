use crate::engine::world::chunk::data::ChunkData;
use crate::engine::world::chunk::local_location::{IndexedLocalLocation, LocalLocation};
use crate::engine::world::chunk::CHUNK_SIZE;
use crate::engine::world::voxel::Voxel;
use cgmath::Vector3;
use noise::{NoiseFn, Perlin};

pub fn get_chunk(world_seed: u32, chunk_position: Vector3<u32>) -> ChunkData {
    generate_perlin_terrain(world_seed, chunk_position)
}

pub fn generate_perlin_terrain(world_seed: u32, chunk_position: Vector3<u32>) -> ChunkData {
    // Create empty chunk data
    let mut chunk_voxel_data = ChunkData::new_with_uniform_type(Voxel::default());

    let mut perlin = Perlin::new(world_seed);

    let octaves = vec![
        NoiseLayer {
            scale: 0.002,
            weight: 1.5,
        },
        NoiseLayer {
            scale: 0.007,
            weight: 0.9,
        },
        NoiseLayer {
            scale: 0.02,
            weight: 0.3,
        },
        NoiseLayer {
            scale: 0.07,
            weight: 0.06,
        },
        NoiseLayer {
            scale: 0.4,
            weight: 0.03,
        },
    ];

    // Fill empty chunk data with randomly selected voxels
    chunk_voxel_data
        .data
        .iter_mut()
        .enumerate()
        .for_each(|(i, v)| {
            let coords = (Vector3::from(LocalLocation::from(i)) + chunk_position * CHUNK_SIZE)
                .cast::<f64>()
                .expect("Conversion from u32 to f64 should be safe");

            let layered_perlin = perlin.get_layered(&octaves, [coords.x, coords.z]);
            let normalized_height = (layered_perlin + 1.0) / 2.0;
            let height = 64.0 * normalized_height + 1.0;

            v.ty = if coords.y < height {
                {
                    if coords.y + 1.0 < height {
                        if coords.y + 6.0 < height {
                            if perlin.get([coords.x * 0.25, coords.y * 0.4, coords.z * 0.25]) < -0.7
                            {
                                4
                            } else if perlin.get([coords.x * 0.3, coords.y * 0.3, coords.z * 0.3])
                                < -0.7
                            {
                                5
                            } else {
                                3
                            }
                        } else {
                            1
                        }
                    } else {
                        2
                    }
                }
            } else {
                0
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
