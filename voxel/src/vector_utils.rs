use cgmath::Vector3;

macro_rules! impl_rem_euclid_elem_wise {
    ($($ty:ty),+) => {
        $(
            impl RemEuclid<$ty> for Vector3<$ty> {
                fn rem_euclid(self, rhs: $ty) -> Self {
                    Vector3::new(self.x.rem_euclid(rhs), self.y.rem_euclid(rhs), self.z.rem_euclid(rhs))
                }
            }
        )+
    };
}

pub trait RemEuclid<T> {
    fn rem_euclid(self, rhs: T) -> Self;
}

impl_rem_euclid_elem_wise!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128);

pub trait MapElemWise<A, T> {
    fn map_elem_wise<F: Fn(A) -> T>(self, f: F) -> Vector3<T>;
}

impl<A, T> MapElemWise<A, T> for Vector3<A> {
    fn map_elem_wise<F: Fn(A) -> T>(self, f: F) -> Vector3<T> {
        Vector3::new(f(self.x), f(self.y), f(self.z))
    }
}

macro_rules! impl_abs_elem_wise {
    ($($ty:ty),+) => {
        $(
            impl AbsValue for Vector3<$ty> {
                fn abs(self) -> Self {
                    Vector3::new(self.x.abs(), self.y.abs(), self.z.abs())
                }
            }
        )+
    };
}

pub trait AbsValue {
    fn abs(self) -> Self;
}

impl_abs_elem_wise!(i8, i16, i32, i64, i128, f32, f64);
