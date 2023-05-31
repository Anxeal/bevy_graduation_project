use bevy::{
    core_pipeline::bloom::BloomSettings,
    log::LogPlugin,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use clap::{arg, command, value_parser};

use color_space::{Lch, ToRgb};

mod client;
mod error;
mod log;
mod plugin;
mod systems;

#[derive(Component)]
struct Shape;
#[derive(Component)]
struct Ghost;
#[derive(Component)]
struct SpawnIndicator;

#[derive(Resource, Clone)]
struct BallData {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource, Default)]
struct BallsSpawned(i32);

#[derive(Resource)]
struct SpawnHeight(f32);

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "client=debug");
    }

    let matches = command!()
        .arg(
            arg!(
                -a --addr <ADDR> "The address to connect to"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -p --port <PORT> "The port to connect to"
            )
            .required(false)
            .value_parser(value_parser!(u16).range(1..=65535)),
        )
        .get_matches();

    let mut app = App::new();

    let now = chrono::Local::now();
    let file_name = format!("log_{}.log", now.format("%Y-%m-%d_%H-%M-%S"));

    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugin(log::LogPlugin {
            file_appender_settings: Some(log::FileAppenderSettings {
                rolling: log::Rolling::Never,
                path: "".into(),
                prefix: file_name.into(),
            }),
            ..default()
        });

    let mut rapier_physics = plugin::RapierPhysicsPlugin::new();

    if let Some(addr) = matches.get_one::<String>("addr") {
        rapier_physics = rapier_physics.with_addr(addr.as_str());
    }

    if let Some(&port) = matches.get_one("port") {
        rapier_physics = rapier_physics.with_port(port);
    }

    app.add_plugin(rapier_physics);

    app.add_startup_system(setup_resources.at_start())
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_system(rotate)
        .add_system(add_ball_on_click)
        .add_system(adjust_spawn_height)
        .add_system(bevy::window::close_on_esc);

    app.insert_resource(ClearColor(Color::rgb(0.9, 0.6, 0.3)))
        .insert_resource(RapierConfiguration {
            gravity: Vec3::new(0.0, -30.0, 0.0),
            ..Default::default()
        })
        .insert_resource(SpawnHeight(5.0))
        .insert_resource(BallsSpawned::default());

    app.run();
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: PerspectiveProjection {
                fov: 50.0_f32.to_radians(),
                ..default()
            }
            .into(),
            transform: Transform::from_xyz(-10.0, 15.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::default(),
    ));
}

const NUM_COLORS: i32 = 16;

fn setup_resources(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut ball_materials = vec![];
    let texture: Handle<Image> = server.load("checkerboard.jpg");
    for i in 0..NUM_COLORS {
        let lch = Lch::new(45f64, 65f64, 360.0 / NUM_COLORS as f64 * i as f64);
        let rgb = lch.to_rgb();
        ball_materials.push(materials.add(StandardMaterial {
            base_color: Color::rgb(
                rgb.r as f32 / 255.0,
                rgb.g as f32 / 255.0,
                rgb.b as f32 / 255.0,
            ),
            base_color_texture: Some(texture.clone()),
            perceptual_roughness: 0.6,
            metallic: 0.2,
            ..default()
        }));
    }
    commands.insert_resource(BallData {
        mesh: meshes.add(
            shape::UVSphere {
                radius: 0.5,
                sectors: 18,
                stacks: 9,
            }
            .into(),
        ),
        materials: ball_materials,
    });
}

fn setup_physics(
    mut commands: Commands,
    ball_data: Res<BallData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    balls_spawned: ResMut<BallsSpawned>,
) {
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(20.0, 2.0, 20.0),
        Vec3::NEG_Y,
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(3.0, 5.0, 3.0),
        Vec3::new(-5.0, 2.0, -7.0),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(10.0, 2.0, 10.0),
        Vec3::new(4.0, 1.0, 4.0),
    );

    spawn_ball(
        &mut commands,
        ball_data.clone(),
        Vec3::Y * 4.0,
        balls_spawned,
    );

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(1.0, 2.0, 3.0))
            .looking_at(Vec3::ZERO, Vec3::Y)
            .with_scale(Vec3::splat(0.2)),
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: ball_data.mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            ..default()
        },
        NotShadowCaster,
        NotShadowReceiver,
        Ghost,
    ));

    commands.spawn((
        PbrBundle {
            mesh: ball_data.mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(0.0, 0.0, 0.0, 0.5),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::new(1.0, 0.1, 1.0)),
            ..default()
        },
        NotShadowCaster,
        NotShadowReceiver,
        SpawnIndicator,
    ));
}

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    size: Vec3,
    position: Vec3,
) {
    commands.spawn((
        Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        Restitution::coefficient(0.5),
        PbrBundle {
            mesh: meshes.add(shape::Box::new(size.x, size.y, size.z).into()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.2, 0.5, 1.0),
                perceptual_roughness: 0.3,
                ..default()
            }),
            transform: Transform::from_translation(position),
            ..default()
        },
    ));
}

fn spawn_ball(
    commands: &mut Commands,
    ball_data: BallData,
    pos: Vec3,
    mut balls_spawned: ResMut<BallsSpawned>,
) {
    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Restitution::coefficient(0.7),
        Shape,
        PbrBundle {
            mesh: ball_data.mesh,
            material: ball_data.materials[(balls_spawned.0 % NUM_COLORS) as usize].clone(),
            transform: Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_x(90_f32.to_radians())),
            ..default()
        },
    ));
    balls_spawned.0 += 1;
}
fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

fn add_ball_on_click(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    ball_data: Res<BallData>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    spawn_height: Res<SpawnHeight>,
    mut ghost_query: Query<&mut Transform, With<Ghost>>,
    mut indicator_query: Query<&mut Transform, (With<SpawnIndicator>, Without<Ghost>)>,
    balls_spawned: ResMut<BallsSpawned>,
) {
    let window = windows.get_primary().unwrap();
    let mouse_position = if let Some(pos) = window.cursor_position() {
        pos
    } else {
        return;
    };

    let (camera_transform, camera) = camera_query.single();

    let mouse_ray = camera
        .viewport_to_world(camera_transform, mouse_position)
        .unwrap();

    let t = -mouse_ray.origin.y / mouse_ray.direction.y;
    let hit_pos = mouse_ray.origin + mouse_ray.direction * t;

    let spawn_pos = hit_pos + Vec3::Y * spawn_height.0;

    ghost_query.single_mut().translation = spawn_pos;
    indicator_query.single_mut().translation = hit_pos;

    if mouse_button_input.just_pressed(MouseButton::Left)
        || mouse_button_input.pressed(MouseButton::Right)
    {
        spawn_ball(&mut commands, ball_data.clone(), spawn_pos, balls_spawned);
    }
}

fn adjust_spawn_height(input: Res<Input<KeyCode>>, mut spawn_height: ResMut<SpawnHeight>) {
    let mut direction: i32 = 0;
    if input.pressed(KeyCode::LShift) {
        direction += 1;
    }
    if input.pressed(KeyCode::LControl) {
        direction -= 1;
    }
    spawn_height.0 = (spawn_height.0 + direction as f32 * 0.25).clamp(1.5, 10.0);
}
