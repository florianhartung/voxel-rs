use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

#[cfg(test)]
mod tests {
    use crate::engine::world::chunk::local_location::LocalLocation;
    use cgmath::Vector3;

    #[test]
    fn a() {
        let loc = LocalLocation::new(1, 2, 3)
            .expect("Location is inside boundaries because it's hardcoded");
        let right = Vector3::unit_x();

        let right_neighbor = loc + right;
        dbg!(right_neighbor);
    }
}
