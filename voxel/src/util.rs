use bytemuck::NoUninit;
use cgmath::Vector3;

pub trait AsBufferData {
    fn as_buffer_data(&self) -> Box<[u8]>;
}

impl<T: NoUninit> AsBufferData for Vector3<T> {
    fn as_buffer_data(&self) -> Box<[u8]> {
        bytemuck::cast_slice(&[self.x, self.y, self.z]).into()
    }
}
