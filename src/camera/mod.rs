use bevy::{prelude::{Plugin, App, Camera2d, Camera, KeyCode, Res, Input, Query, Transform, EventReader}, input::{keyboard::KeyboardInput, mouse::MouseWheel}, time::Time};



pub struct CameraController;

impl Plugin for CameraController {
    fn build(&self, app: &mut App) {
        app.add_system(move_camera);
    }
}


fn move_camera(
    mut camera_q: Query<(&Camera, &mut Transform)>,
    input: Res<Input<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
    time: Res<Time>
) {
    match camera_q.get_single_mut() {
        Ok((camera, mut transform)) => {
            let factor = if input.pressed(KeyCode::LShift) { 2. } else { 1. };
            if input.pressed(KeyCode::W) {
                transform.translation.y += 72. * factor * time.delta_seconds();
            }
            if input.pressed(KeyCode::S) {
                transform.translation.y -= 72. * factor * time.delta_seconds();
            }
            if input.pressed(KeyCode::D) {
                transform.translation.x += 72. * factor * time.delta_seconds();
            }
            if input.pressed(KeyCode::A) {
                transform.translation.x -= 72. * factor * time.delta_seconds();
            }

            for ev in mouse_wheel.iter() {
                match ev.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => {
                        let factor = ev.y / 10.;
                        transform.scale.x = f32::clamp(transform.scale.x - factor, 0.5, 1.25);
                        transform.scale.y = f32::clamp(transform.scale.y - factor, 0.5, 1.25);
                    },
                    bevy::input::mouse::MouseScrollUnit::Pixel => {
                        let factor = ev.y;
                    }
                }
            }
        },
        Err(_) => {}
    }
}