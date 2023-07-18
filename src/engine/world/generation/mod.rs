use crate::engine::world::chunk::data::ChunkData;
use crate::engine::world::chunk::local_location::{IndexedLocalLocation, LocalLocation};
use crate::engine::world::voxel::Voxel;
use cgmath::Vector3;
use noise::{NoiseFn, Perlin};

pub fn get_chunk(world_seed: u32 /*, _pos: Vec */) -> ChunkData {
    generate_perlin_terrain(world_seed)
}

pub fn generate_perlin_terrain(world_seed: u32) -> ChunkData {
    // Create empty chunk data
    let mut chunk_voxel_data = ChunkData::new_with_uniform_type(Voxel::default());

    let mut perlin = Perlin::new(world_seed);

    let octaves = vec![
        NoiseLayer {
            scale: 0.002,
            weight: 1.0,
        },
        NoiseLayer {
            scale: 0.02,
            weight: 0.4,
        },
        NoiseLayer {
            scale: 0.1,
            weight: 0.04,
        },
        NoiseLayer {
            scale: 0.4,
            weight: 0.02,
        },
    ];

    // Fill empty chunk data with randomly selected voxels
    chunk_voxel_data
        .data
        .iter_mut()
        .enumerate()
        .for_each(|(i, v)| {
            let coords = Vector3::from(LocalLocation::from(i))
                .cast::<f64>()
                .expect("Conversion from u32 to f64 should be safe");

            let layered_perlin = perlin.get_layered(&octaves, [coords.x, coords.z]);
            let normalized_height = (layered_perlin + 1.0) / 2.0;
            let height = 40.0 * normalized_height + 5.0;

            v.ty = if coords.y < height { 1 } else { 0 };
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
