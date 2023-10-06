#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::needless_pass_by_value, clippy::cast_precision_loss)]

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    math::vec3,
    prelude::{
        shape::{Box, UVSphere},
        App, AssetServer, Assets, Camera, Camera3dBundle, ClearColor, Color, Commands, Component,
        EventReader, FixedUpdate, Input, KeyCode, Mesh, PbrBundle, PluginGroup, Query, Res, ResMut,
        StandardMaterial, Startup, TextBundle, Transform, Vec3, With, Without,
    },
    text::{Text, TextAlignment, TextStyle},
    ui::{Style, UiRect, Val},
    window::{MonitorSelection, Window, WindowPlugin, WindowPosition},
    DefaultPlugins,
};
use bevy_rapier3d::{
    prelude::{
        ActiveEvents, Collider, CollisionEvent, NoUserData, RapierPhysicsPlugin, RigidBody,
        Velocity,
    },
    render::RapierDebugRenderPlugin,
};
use rand::random;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Platform;

#[derive(Component)]
struct MainCamera {
    offset_from_target: Vec3,
}

#[derive(Component)]
struct Score(u32);

#[derive(Component)]
struct Scored(bool);

#[derive(Component)]
struct ScoreText;

const PLATFORM_SIZE: Vec3 = vec3(10., 3., 100.);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    const CAMERA_OFFSET: Vec3 = vec3(0., 15., 15.);
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
        Score(0),
        ActiveEvents::COLLISION_EVENTS,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Box {
                max_x: PLATFORM_SIZE.x / 2.,
                max_y: PLATFORM_SIZE.y / 2.,
                max_z: PLATFORM_SIZE.z / 2.,
                min_x: -PLATFORM_SIZE.x / 2.,
                min_y: -PLATFORM_SIZE.y / 2.,
                min_z: -PLATFORM_SIZE.z / 2.,
            })),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(
                    random::<f32>().mul_add(10., -5.),
                    -10. - PLATFORM_SIZE.y / 2.,
                    0.,
                ));
                transform.rotate_axis(Vec3::X, -30_f32.to_radians());
                transform
            },
            material: materials.add(Color::BLACK.into()),
            ..Default::default()
        },
        Platform,
        Collider::cuboid(PLATFORM_SIZE.x / 2., PLATFORM_SIZE.y / 2., PLATFORM_SIZE.z / 2.),
        Scored(false),
    ));
    commands.spawn((
        TextBundle::from_section(
            "0",
            TextStyle {
                font: asset_server.load("Fira Code Retina.ttf"),
                font_size: 100.,
                color: Color::GREEN,
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            margin: UiRect::horizontal(Val::Auto),
            ..Default::default()
        }),
        ScoreText,
    ));
}

fn handle_input(mut query: Query<&mut Velocity>, keyboard: Res<Input<KeyCode>>) {
    let horizontal = f32::from(keyboard.any_pressed([KeyCode::Right, KeyCode::D]))
        - f32::from(keyboard.any_pressed([KeyCode::Left, KeyCode::A]));
    query.for_each_mut(|mut velocity| {
        velocity.linvel.x += horizontal * 0.5;
    });
}

fn increase_score_and_spawn_platforms(
    mut collision_events: EventReader<CollisionEvent>,
    mut ball_query: Query<(&mut Score, &Transform)>,
    mut platform_query: Query<&mut Scored>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in &mut collision_events {
        if let CollisionEvent::Started(ball, platform, _) = event {
            let mut scored = platform_query.get_mut(*platform).unwrap();
            if !scored.0 {
                let (mut score, transform) = ball_query.get_mut(*ball).unwrap();
                score.0 += 1;
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(Box {
                            max_x: PLATFORM_SIZE.x / 2.,
                            max_y: PLATFORM_SIZE.y / 2.,
                            max_z: PLATFORM_SIZE.z / 2.,
                            min_x: -PLATFORM_SIZE.x / 2.,
                            min_y: -PLATFORM_SIZE.y / 2.,
                            min_z: -PLATFORM_SIZE.z / 2.,
                        })),
                        transform: {
                            let mut transform = Transform::from_translation(
                                Vec3::new(
                                    random::<f32>().mul_add(10., -5.) + transform.translation.x,
                                    transform.translation.y - 73.,
                                    transform.translation.z - 107.,
                                ),
                            );
                            transform.rotate_axis(Vec3::X, -30_f32.to_radians());
                            transform
                        },
                        material: materials.add(Color::BLACK.into()).clone(),
                        ..Default::default()
                    },
                    Platform,
                    Collider::cuboid(PLATFORM_SIZE.x / 2., PLATFORM_SIZE.y / 2., PLATFORM_SIZE.z / 2.),
                    Scored(false),
                ));
            }
            scored.0 = true;
        }
    }
}

fn update_score(score_query: Query<&Score>, mut text_query: Query<&mut Text, With<ScoreText>>) {
    let score = score_query.single().0;
    text_query.single_mut().sections[0].value = score.to_string();
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

fn limit_ball_speed(score_query: Query<&Score>, mut ball_query: Query<&mut Velocity, With<Ball>>) {
    let score = score_query.single().0;
    let max_speed = (score as f32).mul_add(2., 20.);
    ball_query.for_each_mut(|mut velocity| {
        if velocity.linvel.length() > max_speed {
            velocity.linvel = velocity.linvel.normalize() * max_speed;
        }
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
        .add_systems(
            FixedUpdate,
            (
                handle_input,
                move_camera_to_ball,
                increase_score_and_spawn_platforms,
                update_score,
                limit_ball_speed,
            ),
        )
        .run();
}
