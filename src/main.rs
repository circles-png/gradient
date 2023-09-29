#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::needless_pass_by_value, clippy::cast_precision_loss)]

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    math::{vec2, vec3},
    prelude::{
        shape::{Quad, UVSphere},
        App, Assets, Camera, Camera3dBundle, ClearColor, Color, Commands, Component, FixedUpdate,
        Handle, Input, KeyCode, Mesh, PbrBundle, PluginGroup, Quat, Query, Res, ResMut,
        StandardMaterial, Startup, Transform, Vec3, With, Without, Vec2,
    },
    window::{MonitorSelection, Window, WindowPlugin, WindowPosition},
    DefaultPlugins,
};

#[derive(Component)]
struct Ball {
    acceleration: f32,
    friction: f32,
    max_input_velocity: f32,
    velocity: Vec3,
    gravity: f32,
}

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct MainCamera {
    offset_from_target: Vec3,
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const CAMERA_OFFSET: Vec3 = vec3(0., 7., 15.);
    const GROUND_SIZE: Vec2 = vec2(10., 1000.);
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..Default::default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            transform: Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        BloomSettings::default(),
        MainCamera {
            offset_from_target: CAMERA_OFFSET,
        },
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(UVSphere {
                radius: 1.,
                sectors: 64,
                stacks: 128,
            })),
            material: materials.add(StandardMaterial {
                emissive: Color::rgb_linear(0., 2., 0.),
                ..Default::default()
            }),
            ..Default::default()
        },
        Ball {
            acceleration: 0.02,
            friction: 0.9,
            max_input_velocity: 0.5,
            velocity: Vec3::NEG_Z,
            gravity: 0.01,
        },
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Quad {
                size: GROUND_SIZE,
                ..Default::default()
            })),
            transform: Transform::from_rotation(Quat::from_axis_angle(
                Vec3::X,
                -90_f32.to_radians(),
            ))
            .with_translation(Vec3::new(0., -1., 10. - GROUND_SIZE.y / 2.)),
            material: materials.add(Color::BLACK.into()),
            ..Default::default()
        },
        Ground,
    ));
}

fn handle_input(mut query: Query<&mut Ball>, keyboard: Res<Input<KeyCode>>) {
    let horizontal = f32::from(keyboard.any_pressed([KeyCode::Right, KeyCode::D]))
        - f32::from(keyboard.any_pressed([KeyCode::Left, KeyCode::A]));
    query.for_each_mut(|mut ball| {
        let acceleration = ball.acceleration;
        ball.velocity.x += horizontal * acceleration;
    });
}

fn apply_input_friction(mut query: Query<&mut Ball>) {
    query.for_each_mut(|mut ball| {
        ball.velocity.x *= ball.friction;
        ball.velocity.x = ball
            .velocity
            .x
            .clamp(-ball.max_input_velocity, ball.max_input_velocity);
    });
}

fn move_ball(mut query: Query<(&mut Transform, &Ball)>) {
    query.for_each_mut(|(mut transform, ball)| {
        transform.translation += ball.velocity;
    });
}

type CameraData<'a> = (&'a mut Transform, &'a MainCamera);
type CameraFilter = (With<MainCamera>, Without<Ball>);

fn move_camera_to_ball(
    ball_query: Query<&Transform, With<Ball>>,
    mut camera_query: Query<CameraData, CameraFilter>,
) {
    let ball_position: Vec3 = ball_query
        .iter()
        .map(|transform| transform.translation)
        .sum::<Vec3>()
        / ball_query.iter().len() as f32;
    camera_query.for_each_mut(|(mut transform, main_camera)| {
        transform.translation = transform
            .translation
            .lerp(ball_position + main_camera.offset_from_target, 0.05);
        let target = (*transform).looking_at(ball_position, Vec3::Y).rotation;
        transform.rotation = transform.rotation.slerp(target, 0.05);
    });
}

type GroundData<'a> = (&'a Transform, &'a Handle<Mesh>);
type GroundFilter = (With<Ground>, Without<Ball>);

fn apply_gravity(
    mut ball_query: Query<(&mut Ball, &mut Transform, &Handle<Mesh>)>,
    ground_query: Query<GroundData, GroundFilter>,
    meshes: Res<Assets<Mesh>>,
) {
    let ground_y = ground_query
        .iter()
        .map(|(transform, _)| transform.translation.y)
        .sum::<f32>()
        / ground_query.iter().len() as f32;
    let ground_mesh = meshes.get(ground_query.iter().next().unwrap().1).unwrap();
    let ground_translation = ground_query.iter().next().unwrap().0.translation;
    ball_query.for_each_mut(|(mut ball, mut transform, handle)| {
        ball.velocity.y -= ball.gravity;
        let ball_extents = meshes
            .get(handle)
            .unwrap()
            .compute_aabb()
            .unwrap()
            .half_extents;
        if transform.translation.y > ground_y
            && transform.translation.y < ground_y + ball_extents.y
            && {
                let bounding_box = ground_mesh.compute_aabb().unwrap();
                let front = bounding_box.min().y + ground_translation.z;
                let back = bounding_box.max().y + ground_translation.z;
                let left = bounding_box.min().x - ground_translation.x;
                let right = bounding_box.max().x - ground_translation.x;
                transform.translation.x > left - ball_extents.x
                    && transform.translation.x < right + ball_extents.x
                    && transform.translation.z > front - ball_extents.z
                    && transform.translation.z < back + ball_extents.z
            }
        {
            transform.translation.y = ground_y + ball_extents.y;
            ball.velocity.y = 0.;
        }
        if transform.translation.y < ground_y {
            ball.velocity.x = 0.;
            ball.velocity.z = 0.;
        }
    });
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "gradient".to_string(),
                position: WindowPosition::Centered(MonitorSelection::Index(1)),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(
            FixedUpdate,
            (
                handle_input,
                move_ball,
                apply_input_friction,
                move_camera_to_ball,
                apply_gravity,
            ),
        )
        .run();
}
