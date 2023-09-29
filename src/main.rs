#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::needless_pass_by_value, clippy::cast_precision_loss)]

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    math::vec3,
    prelude::{
        shape::{Box, UVSphere},
        App, Assets, Camera, Camera3dBundle, ClearColor, Color, Commands, Component, FixedUpdate,
        Input, KeyCode, Mesh, PbrBundle, PluginGroup, Query, Res, ResMut, StandardMaterial,
        Startup, Transform, Vec3, With, Without,
    },
    window::{MonitorSelection, Window, WindowPlugin, WindowPosition},
    DefaultPlugins,
};
use bevy_rapier3d::{
    prelude::{Collider, NoUserData, RapierPhysicsPlugin, RigidBody, Velocity},
    render::RapierDebugRenderPlugin,
};
use rand::random;

#[derive(Component)]
struct Ball;

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
    const GROUND_SIZE: Vec3 = vec3(10., 3., 100.);
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
        Ball,
        RigidBody::Dynamic,
        Velocity::zero(),
        Collider::ball(1.),
    ));
    let ground_material = materials.add(Color::BLACK.into());
    for index in 0..5 {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Box {
                    max_x: GROUND_SIZE.x / 2.,
                    max_y: GROUND_SIZE.y / 2.,
                    max_z: GROUND_SIZE.z / 2.,
                    min_x: -GROUND_SIZE.x / 2.,
                    min_y: -GROUND_SIZE.y / 2.,
                    min_z: -GROUND_SIZE.z / 2.,
                })),
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(
                        random::<f32>().mul_add(5., -2.5),
                        (index as f32).mul_add(-30., -10. - GROUND_SIZE.y / 2.),
                        (index as f32) * -120.,
                    ));
                    println!("transform: {:?}", transform.translation);
                    transform.rotate_axis(Vec3::X, -5_f32.to_radians());
                    transform
                },
                material: ground_material.clone(),
                ..Default::default()
            },
            Ground,
            Collider::cuboid(GROUND_SIZE.x / 2., GROUND_SIZE.y / 2., GROUND_SIZE.z / 2.),
        ));
    }
}

fn handle_input(mut query: Query<&mut Velocity>, keyboard: Res<Input<KeyCode>>) {
    let horizontal = f32::from(keyboard.any_pressed([KeyCode::Right, KeyCode::D]))
        - f32::from(keyboard.any_pressed([KeyCode::Left, KeyCode::A]));
    query.for_each_mut(|mut velocity| {
        velocity.linvel.x += horizontal * 0.5;
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

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "gradient".to_string(),
                    position: WindowPosition::Centered(MonitorSelection::Index(1)),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(FixedUpdate, (handle_input, move_camera_to_ball))
        .run();
}
