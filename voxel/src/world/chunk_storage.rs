use std::cell::Cell;

use hashbrown::HashMap;

use crate::world::chunk::Chunk;
use crate::world::location::ChunkLocation;

struct ChunkStorage {
    chunks: HashMap<ChunkLocation, StoredChunk>,
}

struct StoredChunk {
    data: Chunk,
    ref_counter: Cell<usize>,
}

struct ChunkRef {}
