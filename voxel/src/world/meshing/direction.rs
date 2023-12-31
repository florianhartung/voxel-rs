use cgmath::Vector3;
use strum_macros::EnumIter;

#[derive(EnumIter, Copy, Clone, Debug)]
pub enum Direction {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}

impl Direction {
    pub fn to_vec(self) -> Vector3<i32> {
        match self {
            Direction::XPos => Vector3::unit_x(),
            Direction::XNeg => -Vector3::unit_x(),
            Direction::YPos => Vector3::unit_y(),
            Direction::YNeg => -Vector3::unit_y(),
            Direction::ZPos => Vector3::unit_z(),
            Direction::ZNeg => -Vector3::unit_z(),
        }
    }

    pub fn get_normal_axes(self) -> (Vector3<i32>, Vector3<i32>) {
        let v = self.to_vec();

        (Vector3::new(v.y, v.z, v.x), Vector3::new(v.z, v.x, v.y))
    }
}

impl From<Direction> for Vector3<i32> {
    fn from(value: Direction) -> Self {
        value.to_vec()
    }
}
