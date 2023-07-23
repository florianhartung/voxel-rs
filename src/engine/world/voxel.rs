use cgmath::{Vector3, Zero};

#[derive(Copy, Clone, Debug, Default)]
pub struct Voxel {
    pub ty: u8,
}

impl Voxel {
    pub fn new(ty: u8) -> Self {
        Self { ty }
    }

    pub fn color(&self) -> Vector3<f32> {
        const ENABLE_RAND_COLORS: bool = true;
        match self.ty {
            0 => Vector3::zero(),
            1 => {
                Vector3::new(0.15, 0.1, 0.07)
                    + if ENABLE_RAND_COLORS {
                        fastrand::f32() * Vector3::new(0.1, 0.07, 0.05)
                    } else {
                        Vector3::zero()
                    }
            }
            2 => {
                Vector3::new(0.2, 0.41, 0.1)
                    + if ENABLE_RAND_COLORS {
                        fastrand::f32() * Vector3::new(0.1, 0.2, 0.08)
                    } else {
                        Vector3::zero()
                    }
            }
            3 => {
                Vector3::new(0.4, 0.4, 0.4)
                    + if ENABLE_RAND_COLORS {
                        fastrand::f32() * Vector3::new(0.2, 0.2, 0.2)
                    } else {
                        Vector3::zero()
                    }
            }
            4 => Vector3::new(0.02, 0.02, 0.04),
            5 => Vector3::new(1.0, 0.5, 0.0),
            _ => Vector3::new(1.0, 0.0, 1.0),
        }
    }
}
