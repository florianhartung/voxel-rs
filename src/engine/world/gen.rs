use crate::engine::world::chunk::ChunkData;
use crate::engine::world::voxel::Voxel;

pub fn get_chunk(/*_pos: Vec */) -> ChunkData {
    generate_random_chunk_data()
}

pub fn generate_random_chunk_data() -> ChunkData {
    // Create empty chunk data
    let mut chunk_voxel_data = ChunkData::new_with_uniform_type(Voxel::default());

    // Fill empty chunk data with randomly selected voxels
    chunk_voxel_data
        .data
        .iter_mut()
        .enumerate()
        .for_each(|(_i, v)| v.ty = if fastrand::f32() < 0.1 { 1 } else { 0 });

    chunk_voxel_data
}
