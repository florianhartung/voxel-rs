use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use lazy_static::lazy_static;
use noise::NoiseFn;

lazy_static! {
    static ref POSITIONS: Vec<ChunkLocation> = iproduct(-3..3, -3..3, -3..3)
        .map(|(x, y, z)| ChunkLocation::new(x, y, z))
        .collect();
    static ref CHUNK_GEN: WorldGenerator = WorldGenerator::new(0);
}

fn gen_chunks() -> Vec<ChunkData> {
    POSITIONS
        .iter()
        .map(|pos| CHUNK_GEN.get_chunk_data_at(pos))
        .collect_vec()
}

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("noise", |b| b.iter(|| noise(black_box(123.0), black_box(321.0), black_box(333.0))));
    // c.bench_function("bracket", |b| {
    //     b.iter(|| bracket(black_box(123.0), black_box(321.0), black_box(333.0)))
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
