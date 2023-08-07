use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Add, Deref, DerefMut, Sub};

use cgmath::Vector3;
use itertools::iproduct;

use crate::engine::vector_utils::RemEuclid;
use crate::engine::world::CHUNK_SIZE;

/// An absolute location in the world. It contains a chunk location and a local chunk location encoded into a single Vector3.
#[derive(Copy, Clone, Debug)]
pub struct WorldLocation(pub Vector3<i32>);

impl WorldLocation {
    pub fn new(chunk_location: ChunkLocation, local_location: LocalChunkLocation) -> Self {
        Self(chunk_location.0 * CHUNK_SIZE as i32 + local_location.location)
    }

    pub fn separate(self) -> (ChunkLocation, LocalChunkLocation<WithinBounds>) {
        let mut chunk_location = ChunkLocation::new(self.0 / CHUNK_SIZE as i32);
        if self.0.x < 0 {
            chunk_location.0.x -= 1;
        }
        if self.0.y < 0 {
            chunk_location.0.y -= 1;
        }
        if self.0.z < 0 {
            chunk_location.0.z -= 1;
        }
        let local_chunk_location = LocalChunkLocation::new_unchecked(self.0.rem_euclid(CHUNK_SIZE as i32));

        (chunk_location, local_chunk_location)
    }

    pub fn to_f32(self) -> Vector3<f32> {
        self.0
            .cast()
            .expect("Conversion from i32 to f32 is safe and should never fail")
    }
}

/// The location of a specific chunk in the world.
/// Each ChunkLocation unit will be equal to one CHUNK_SIZE when rendering.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ChunkLocation(Vector3<i32>);

impl ChunkLocation {
    pub fn new<T: Into<i32>>(location: Vector3<T>) -> Self {
        Self(Vector3::new(location.x.into(), location.y.into(), location.z.into()))
    }

    pub fn from_world_location_f32(location: Vector3<f32>) -> Self {
        Self(Vector3::new(
            (location.x.floor() as i32) / CHUNK_SIZE as i32,
            (location.y.floor() as i32) / CHUNK_SIZE as i32,
            (location.z.floor() as i32) / CHUNK_SIZE as i32,
        ))
    }

    pub fn from_world_location_f64(location: Vector3<f64>) -> Self {
        Self(Vector3::new(
            (location.x.floor() as i32) / CHUNK_SIZE as i32,
            (location.y.floor() as i32) / CHUNK_SIZE as i32,
            (location.z.floor() as i32) / CHUNK_SIZE as i32,
        ))
    }

    pub fn to_world_location_f32(self) -> Vector3<f32> {
        let scaled = self.0 * (CHUNK_SIZE as i32);
        Vector3::new(scaled.x as f32, scaled.y as f32, scaled.z as f32)
    }

    pub fn to_world_location_f64(self) -> Vector3<f64> {
        let scaled = self.0 * (CHUNK_SIZE as i32);
        Vector3::new(scaled.x as f64, scaled.y as f64, scaled.z as f64)
    }
}

impl Add for ChunkLocation {
    type Output = ChunkLocation;

    fn add(self, rhs: Self) -> Self::Output {
        ChunkLocation::new(self.0 + rhs.0)
    }
}

impl Sub for ChunkLocation {
    type Output = ChunkLocation;

    fn sub(self, rhs: Self) -> Self::Output {
        ChunkLocation::new(self.0 - rhs.0)
    }
}

impl Deref for ChunkLocation {
    type Target = Vector3<i32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChunkLocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A local location inside of a specific chunk.
/// The generic type `State` signals whether it is confirmed that the location is within the chunk boundaries defined by [CHUNK_SIZE].
/// It can be either one of [WithinBounds] or [OutsideBounds].
/// When creating a world object, the State=OutsideBounds is assumed. To get a State=WithinBounds the method [LocalChunkLocation::try_into_checked] can be called.
#[derive(Copy, Clone, Debug)]
pub struct LocalChunkLocation<State = OutsideBounds> {
    location: Vector3<i32>,
    phantom: PhantomData<State>,
}

/// Marker type for [LocalChunkLocation]
/// It known for the local location to be within the chunk boundaries.
#[derive(Copy, Clone, Debug)]
pub struct WithinBounds;

/// Marker type for [LocalChunkLocation]
/// It is unknown whether the local location is within the chunk boundaries.
#[derive(Copy, Clone, Debug)]
pub struct OutsideBounds;

impl LocalChunkLocation {
    pub fn iter() -> impl Iterator<Item = LocalChunkLocation<WithinBounds>> {
        iproduct!(0..(CHUNK_SIZE as i32), 0..(CHUNK_SIZE as i32), 0..(CHUNK_SIZE as i32))
            .map(|coords| LocalChunkLocation::new_unchecked(coords.into()))
    }
}

impl LocalChunkLocation<OutsideBounds> {
    pub fn new(location: Vector3<i32>) -> Self {
        Self {
            location,
            phantom: PhantomData,
        }
    }

    pub fn try_into_checked(self) -> Option<LocalChunkLocation<WithinBounds>> {
        self.location
            .cast::<u8>()
            .filter(|&v| Self::validate_boundaries(v))
            .map(|_| LocalChunkLocation::new_unchecked(self.location))
    }

    fn validate_boundaries(location: Vector3<u8>) -> bool {
        (0..CHUNK_SIZE).contains(&location.x.into())
            && (0..CHUNK_SIZE).contains(&location.y.into())
            && (0..CHUNK_SIZE).contains(&location.z.into())
    }
}

impl<T> LocalChunkLocation<T> {
    pub fn to_f32(self) -> Vector3<f32> {
        Vector3::new(self.location.x as f32, self.location.y as f32, self.location.z as f32)
    }

    pub fn to_f64(self) -> Vector3<f64> {
        Vector3::new(self.location.x as f64, self.location.y as f64, self.location.z as f64)
    }
}

impl<T, A: Into<Vector3<i32>>> Add<A> for LocalChunkLocation<T> {
    type Output = LocalChunkLocation<OutsideBounds>;

    fn add(self, rhs: A) -> Self::Output {
        LocalChunkLocation::new(self.location + rhs.into())
    }
}

impl LocalChunkLocation<WithinBounds> {
    pub fn new_unchecked(location: Vector3<i32>) -> Self {
        LocalChunkLocation {
            location,
            phantom: PhantomData,
        }
    }

    pub fn into_unknown(self) -> LocalChunkLocation<OutsideBounds> {
        LocalChunkLocation {
            location: self.location,
            phantom: PhantomData,
        }
    }
}

impl<T> Deref for LocalChunkLocation<T> {
    type Target = Vector3<i32>;

    fn deref(&self) -> &Self::Target {
        &self.location
    }
}

impl<T> DerefMut for LocalChunkLocation<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.location
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use crate::engine::world::location::{ChunkLocation, LocalChunkLocation, WorldLocation};
    use crate::engine::world::CHUNK_SIZE;

    #[test]
    fn test_world_location() {
        assert!(CHUNK_SIZE == 32, "test only works for chunksize=32");

        let local = LocalChunkLocation::new(Vector3::new(5, 6, 7));
        let chunk = ChunkLocation::new(Vector3::new(1, 2, 3));

        assert_eq!(
            WorldLocation::new(chunk, local).0,
            Vector3::new(1 * CHUNK_SIZE as i32 + 5, 2 * CHUNK_SIZE as i32 + 6, 3 * CHUNK_SIZE as i32 + 7)
        );
        assert_eq!(WorldLocation::new(chunk, local).separate().0 .0, chunk.0);
        assert_eq!(
            WorldLocation::new(chunk, local)
                .separate()
                .1
                .location,
            local.location
        );

        let local_outside = LocalChunkLocation::new(Vector3::new(-1, 0, 0));

        assert_eq!(
            WorldLocation::new(chunk, local_outside).0,
            Vector3::new(CHUNK_SIZE as i32 - 1, 2 * CHUNK_SIZE as i32, 3 * CHUNK_SIZE as i32)
        );

        let negative_world_location = WorldLocation(Vector3::new(-1, -65, 1));
        assert_eq!(negative_world_location.separate().0 .0, Vector3::new(-1, -3, 0));
        assert_eq!(
            negative_world_location.separate().1.location,
            Vector3::new(CHUNK_SIZE as i32 - 1, CHUNK_SIZE as i32 - 1, 1)
        );
    }
}
