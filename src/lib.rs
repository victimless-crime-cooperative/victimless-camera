use bevy::prelude::*;

#[derive(Default)]
pub struct VictimlessCameraPlugin(CameraSettings);

impl VictimlessCameraPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_x_sensitivity(mut self, value: f32) -> Self {
        self.0.x_sensitivity = value;
        self
    }

    pub fn with_y_sensitivity(mut self, value: f32) -> Self {
        self.0.y_sensitivity = value;
        self
    }

    pub fn with_smoothing(mut self, value: f32) -> Self {
        self.0.smoothing = value;
        self
    }

    pub fn with_x_limits(mut self, min: f32, max: f32) -> Self {
        self.0.x_limits = RotationLimits(min, max);
        self
    }
}

impl Plugin for VictimlessCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0)
            .insert_resource(MovementCompass::default())
            .add_event::<RotateCameraEvent>()
            .register_type::<MainCamera>()
            .register_type::<CameraAnchor>()
            .add_systems(
                Update,
                (translate_camera, rotate_camera, read_camera_rotation_inputs),
            )
            .observe(update_compass);
    }
}

/// A utility resource that gives the current direction the camera is facing, as well as it's
/// position
#[derive(Resource, Default)]
pub struct MovementCompass(pub Quat, pub Vec3);

impl MovementCompass {
    /// Returns an input Vector (x, y) direction in 3d space (along the x and z-axis)
    pub fn interpolate_direction(&self, input: Vec2) -> Vec3 {
        (self.0 * Vec3::new(input.x, 0.0, -input.y)).normalize_or_zero()
    }

    /// Returns the forward vector of the compass/camera
    pub fn direction(&self) -> Dir3 {
        Transform::from_rotation(self.0).forward()
    }

    /// Returns the forward vector of the compass/camera
    pub fn position(&self) -> Vec3 {
        self.1
    }
}

/// Used for clamping camera angles, (min, max)
#[derive(Clone, Copy, Default)]
pub struct RotationLimits(pub f32, pub f32);

impl RotationLimits {
    pub fn clamp(&self, value: f32) -> f32 {
        let as_degrees = value.to_degrees();

        if as_degrees < self.0 {
            return self.0.to_radians();
        }

        if as_degrees > self.1 {
            return self.1.to_radians();
        }

        value
    }
}

///
#[derive(Resource, Default, Clone, Copy)]
pub struct CameraSettings {
    pub x_sensitivity: f32,
    pub y_sensitivity: f32,
    pub smoothing: f32,
    pub head_position: Vec3,
    pub x_limits: RotationLimits,
    x_angle: f32,
    y_angle: f32,
}

impl CameraSettings {
    fn rotate_x(&mut self, value: f32) {
        self.x_angle = self.x_limits.clamp(self.x_angle + value);
    }

    fn rotate_y(&mut self, value: f32) {
        self.y_angle = self.y_angle + value;
    }
}

/// Simple event that accepts x any y inputs to rotate the camera
#[derive(Event)]
pub struct RotateCameraEvent(pub Vec2);

#[derive(Event)]
struct CameraOrientation {
    pub rotation: Quat,
    pub translation: Vec3,
}

/// Marker component for the primary camera in the scene
#[derive(Component, Reflect)]
pub struct MainCamera;

/// Component for the object the camera should follow, with an optional offset
#[derive(Component, Default, Reflect)]
pub struct CameraAnchor(pub Option<Vec3>);

fn read_camera_rotation_inputs(
    mut camera_settings: ResMut<CameraSettings>,
    time: Res<Time>,
    mut rotate_events: EventReader<RotateCameraEvent>,
) {
    for event in rotate_events.read() {
        let Vec2 { x, y } = event.0;
        let x_sensitivty = camera_settings.x_sensitivity;
        let y_sensitivity = camera_settings.y_sensitivity;

        if x != 0.0 {
            camera_settings
                .rotate_y(-x * 360.0_f32.to_radians() * time.delta_seconds() * x_sensitivty);
        }

        if y != 0.0 {
            camera_settings
                .rotate_x(-y * 360.0_f32.to_radians() * time.delta_seconds() * y_sensitivity);
        }
    }
}

fn translate_camera(
    camera_settings: Res<CameraSettings>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    anchor_query: Query<(&Transform, &CameraAnchor), Without<MainCamera>>,
) {
    if let Ok(mut camera_transform) = camera_query.get_single_mut() {
        if let Ok((anchor_transform, anchor)) = anchor_query.get_single() {
            if let Some(offset) = anchor.0 {
                let desired_rotation = Quat::from_rotation_y(camera_settings.y_angle)
                    * Quat::from_rotation_x(camera_settings.x_angle);

                let camera_position = (desired_rotation * offset) + anchor_transform.translation;
                camera_transform.translation = camera_transform.translation.lerp(
                    camera_position,
                    time.delta_seconds() * camera_settings.smoothing,
                );
            } else {
                camera_transform.translation = camera_transform.translation.lerp(
                    anchor_transform.translation,
                    time.delta_seconds() * camera_settings.smoothing,
                );
            }
        }
    }
}

fn rotate_camera(
    mut commands: Commands,
    time: Res<Time>,
    camera_settings: Res<CameraSettings>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    if let Ok(mut transform) = query.get_single_mut() {
        let desired_rotation = Quat::from_rotation_y(camera_settings.y_angle)
            * Quat::from_rotation_x(camera_settings.x_angle);

        transform.rotation = transform.rotation.slerp(
            desired_rotation,
            time.delta_seconds() * camera_settings.smoothing,
        );
        commands.trigger(CameraOrientation {
            translation: transform.translation,
            rotation: transform.rotation,
        });
    }
}

fn update_compass(trigger: Trigger<CameraOrientation>, mut compass: ResMut<MovementCompass>) {
    compass.0 = trigger.event().rotation;
    compass.1 = trigger.event().translation;
}
