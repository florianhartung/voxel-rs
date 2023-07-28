use std::time::Duration;

use cgmath::num_traits::FloatConst;
use cgmath::{InnerSpace, Matrix4, Point3, Rad, Vector3};
use wgpu::util::DeviceExt;
use wgpu::BindingType;
use winit::event::{ElementState, VirtualKeyCode};

use crate::engine::rendering::RenderCtx;

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f64>,
    pitch: Rad<f64>,
    projection: Projection,

    raw: RawCamera,
    buffer: wgpu::Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new<V, Y, P, F>(
        render_ctx: &RenderCtx,
        position: V,
        yaw: Y,
        pitch: P,
        width: u32,
        height: u32,
        fov_y: F,
        z_near: f32,
        z_far: f32,
    ) -> Self
    where
        V: Into<Point3<f32>>,
        Y: Into<Rad<f64>>,
        P: Into<Rad<f64>>,
        F: Into<Rad<f32>>,
    {
        let raw = RawCamera {
            view_proj: [[0.0f32; 4]; 4],
        };

        let buffer = render_ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[raw]),
            });

        let bind_group_layout = render_ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::VERTEX,
                    count: None,
                }],
            });

        let bind_group = render_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera bind group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Camera {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            projection: Projection::new(width, height, fov_y, z_near, z_far),
            raw,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn update_buffer(&mut self, render_ctx: &RenderCtx) {
        let (sin_pitch, cos_pitch) = (self.pitch.0 as f32).sin_cos();
        let (sin_yaw, cos_yaw) = (self.yaw.0 as f32).sin_cos();

        let view = Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw)
                .normalize()
                .into(),
            Vector3::unit_y(),
        );
        let proj = self.projection.build_proj_matrix();

        self.raw.view_proj = (proj * view).into();

        render_ctx
            .queue
            .write_buffer(&self.buffer, 0 as _, bytemuck::cast_slice(&[self.raw]));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }
}

pub struct Projection {
    aspect: f32,
    fov_y: Rad<f32>,
    z_near: f32,
    z_far: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fov_y: F, z_near: f32, z_far: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fov_y: fov_y.into(),
            z_near,
            z_far,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn build_proj_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fov_y, self.aspect, self.z_near, self.z_far)
    }
}

pub struct CameraController {
    right: bool,
    left: bool,
    forward: bool,
    backward: bool,
    up: bool,
    down: bool,
    last_rotate_horizontal: f64,
    last_rotate_vertical: f64,
    rotate_horizontal: f64,
    rotate_vertical: f64,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            left: false,
            right: false,
            forward: false,
            backward: false,
            up: false,
            down: false,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            last_rotate_horizontal: 0.0,
            last_rotate_vertical: 0.0,
        }
    }

    pub fn process_keyboard(&mut self, key: &VirtualKeyCode, state: &ElementState) -> bool {
        let is_pressed = matches!(state, ElementState::Pressed);

        use VirtualKeyCode::{LShift, Space, A, D, S, W};
        match key {
            W => {
                self.forward = is_pressed;
                true
            }
            S => {
                self.backward = is_pressed;
                true
            }
            A => {
                self.left = is_pressed;
                true
            }
            D => {
                self.right = is_pressed;
                true
            }
            Space => {
                self.up = is_pressed;
                true
            }
            LShift => {
                self.down = is_pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx;
        self.rotate_vertical = mouse_dy;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos as f32, 0.0, yaw_sin as f32).normalize();
        let right = Vector3::new(-yaw_sin as f32, 0.0, yaw_cos as f32).normalize();

        let forward_speed = if self.forward { self.speed } else { 0.0 } + if self.backward { -self.speed } else { 0.0 };
        let right_speed = if self.right { self.speed } else { 0.0 } + if self.left { -self.speed } else { 0.0 };

        camera.position += forward * forward_speed * dt;
        camera.position += right * right_speed * dt;

        camera.position.y += if self.up { self.speed * dt } else { 0.0 } + if self.down { -self.speed * dt } else { 0.0 };

        const FACTOR: f64 = 0.5;

        camera.yaw += Rad(FACTOR * self.rotate_horizontal + self.last_rotate_horizontal) * self.sensitivity as f64 * dt as f64;
        camera.pitch += Rad(FACTOR * (-self.rotate_vertical) + -self.last_rotate_vertical) * self.sensitivity as f64 * dt as f64;

        self.last_rotate_horizontal = (1.0 - FACTOR) * self.rotate_horizontal;
        self.last_rotate_vertical = (1.0 - FACTOR) * self.rotate_vertical;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep camera's angle from going to far
        let safe_frac_pi_2 = f64::FRAC_PI_2() - 0.001;
        if camera.pitch < -Rad(safe_frac_pi_2) {
            camera.pitch = -Rad(safe_frac_pi_2);
        } else if camera.pitch > Rad(safe_frac_pi_2) {
            camera.pitch = Rad(safe_frac_pi_2);
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawCamera {
    pub view_proj: [[f32; 4]; 4],
}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
