use glam::{vec3, vec4, Mat4, Vec3, Vec4};
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

use crate::scene::SceneUniformData;

pub struct Camera {
    eye: Vec3,
    front: Vec3,
    up: Vec3,
    pitch: f32,
    yaw: f32,

    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: vec3(0.0, 3.0, -3.0),
            front: (vec3(0.0, 0.0, 0.0) - vec3(0.0, 3.0, -3.0)).normalize(),
            up: vec3(0.0, 1.0, 0.0),
            pitch: 0.0,
            yaw: 90.0,

            fov_y_radians: 90.0,
            aspect_ratio: 1600.0 / 900.0,
            z_near: 0.01,
            z_far: 100.0,
        }
    }
}

impl Camera {
    pub fn build_uniforms(&self) -> SceneUniformData {
        let perspective = Mat4::perspective_lh(
            self.fov_y_radians,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );
        let inverse_perspective = perspective.inverse();

        let view = Mat4::look_to_lh(self.eye, self.front, self.up);
        let inverse_view = view.inverse();

        SceneUniformData {
            perspective,
            view,
            inverse_perspective,
            inverse_view,
            camera_position: self.eye,
            aspect_ratio: self.aspect_ratio,
        }
    }
}

pub trait CameraController {
    fn input(&mut self, event: &WindowEvent);
    fn update(&mut self, camera: &mut Camera);
    fn ui(&mut self, camera: &mut Camera, ui: &mut egui::Ui);
}

pub struct FlyCamera {
    direction: Vec3,
    max_speed: f32,

    right_click: bool,
    first_mouse: bool,
    last_x: f32,
    last_y: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl FlyCamera {
    pub fn new() -> Self {
        FlyCamera {
            direction: vec3(0.0, 0.0, 0.0),
            max_speed: 10.0,

            right_click: false,
            first_mouse: false,
            last_x: 0.0,
            last_y: 0.0,
            pitch: 0.0,
            yaw: 90.0,
        }
    }
}
impl CameraController for FlyCamera {
    fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.direction.z = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.direction.x = if is_pressed { -1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.direction.z = if is_pressed { -1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.direction.x = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::E => {
                        self.direction.y = if is_pressed { 1.0 } else { 0.0 };
                    }
                    VirtualKeyCode::Q => {
                        self.direction.y = if is_pressed { -1.0 } else { 0.0 };
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button: MouseButton::Right,
                modifiers: _,
            } => {
                if *state == ElementState::Pressed {
                    self.right_click = true;
                } else {
                    self.right_click = false;
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
                modifiers: _,
            } => {
                if !self.right_click {
                    self.first_mouse = true;
                    return;
                }

                if self.first_mouse {
                    self.last_x = position.x as f32;
                    self.last_y = position.y as f32;
                    self.first_mouse = false;
                }

                let mut x_offset: f32 = position.x as f32 - self.last_x;
                let mut y_offset: f32 = position.y as f32 - self.last_y;
                self.last_x = position.x as f32;
                self.last_y = position.y as f32;

                let sensitivity = 0.2_f32;
                x_offset *= sensitivity;
                y_offset *= sensitivity;
                self.yaw -= x_offset;
                self.pitch -= y_offset;

                self.pitch = self.pitch.clamp(-89.0, 89.0);
                self.direction = vec3(
                    f32::cos(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                    f32::sin(self.pitch.to_radians()),
                    f32::sin(self.yaw.to_radians()) * f32::cos(self.pitch.to_radians()),
                );
            }
            _ => {}
        }
    }

    fn update(&mut self, camera: &mut Camera) {
        camera.front = self.direction;

        camera.eye += camera.front * self.direction.z * self.max_speed;
        camera.eye += Vec3::normalize(Vec3::cross(camera.up, camera.front))
            * self.direction.x
            * self.max_speed;

        let right = Vec3::normalize(Vec3::cross(camera.up, camera.front));
        camera.eye +=
            Vec3::normalize(Vec3::cross(camera.front, right)) * self.direction.y * self.max_speed;
    }

    fn ui(&mut self, camera: &mut Camera, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Camera").show(ui, |ui| {
            ui.label(format!(
                "Position: {:.3} {:.3} {:.3}\nYaw: {:.3}\nPitch: {:.3}",
                camera.eye.x, camera.eye.y, camera.eye.z, self.yaw, self.pitch
            ));

            ui.add(
                egui::Slider::new(&mut self.max_speed, 0.0..=10.0)
                    .text("Camera speed")
                    .show_value(true),
            );

            ui.add(
                egui::Slider::new(&mut camera.fov_y_radians, 10.0..=140.0)
                    .text("FOV (y rad)")
                    .show_value(true),
            );

            ui.add(
                egui::Slider::new(&mut camera.aspect_ratio, 0.0..=3.0)
                    .text("Aspect ratio")
                    .show_value(true),
            );

            ui.add(
                egui::Slider::new(&mut camera.z_near, 0.0..=1.0)
                    .text("Z near")
                    .show_value(true),
            );

            ui.add(
                egui::Slider::new(&mut camera.z_far, 0.0..=100.0)
                    .text("Z far")
                    .show_value(true),
            );
        });
    }
}
