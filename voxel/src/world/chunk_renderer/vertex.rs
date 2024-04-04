use std::fmt::Debug;

use bytemuck::{Pod, Zeroable};
use cgmath::num_traits::ToPrimitive;
use cgmath::Vector3;
use wgpu::vertex_attr_array;

/// Layout:
/// 0: u32
///   - x: u8
///   - y: u8
///   - z: u8
///   - r: u8
/// 1: u32
///   - g: u8
///   - b: u8
///   - normal: 3 bits:  0, 1, 2, 3, 4, 5 => (0, 0, 1), (0, 1, 0), (1, 0, 0), (0, 0, -1), (0, -1, 0), (-1, 0, 0)
///   - ao: 2 bits
///   - _unused: 11 bits
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position_x_y_z_color_r: u32,
    color_g_b_normal_ao: u32,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>, direction: Vector3<f32>, ambient_occlusion: f32) -> Self {
        let x: u8 = position.x.to_u8().unwrap();
        let y: u8 = position.y.to_u8().unwrap();
        let z: u8 = position.z.to_u8().unwrap();
        let r: u8 = (255.0 * color.x).to_u8().unwrap();
        let g: u8 = (255.0 * color.y).to_u8().unwrap();
        let b: u8 = (255.0 * color.z).to_u8().unwrap();

        let normal: u32 = match &direction[..] {
            &[0.0, 0.0, 1.0] => 0,
            &[0.0, 1.0, 0.0] => 1,
            &[1.0, 0.0, 0.0] => 2,
            &[0.0, 0.0, -1.0] => 3,
            &[0.0, -1.0, 0.0] => 4,
            &[-1.0, 0.0, 0.0] => 5,
            _ => panic!("invalid direction"),
        };

        let ao: u32 = ambient_occlusion.to_u32().unwrap();
        assert!((0..=3).contains(&ao));

        Self {
            position_x_y_z_color_r: u32::from_be_bytes([x, y, z, r]),
            color_g_b_normal_ao: u32::from_be_bytes([g, b, 0, 0]) | normal << 13 | ao << 11,
        }
    }

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] = vertex_attr_array![0 => Uint32, 1 => Uint32];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            attributes: &ATTRIBUTES,
            step_mode: wgpu::VertexStepMode::Vertex,
        }
    }
}
