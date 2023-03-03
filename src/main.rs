use bevy::prelude::*;
use bevy_flycam::{FlyCam, NoCameraPlayerPlugin};
use bevy_rapier3d::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct TransientForce {
    force: Vect,
    torque: Vect,
}

impl TransientForce {
    pub fn apply(&self, external_force: &mut ExternalForce) {
        external_force.force += self.force;
        external_force.torque += self.torque;
    }

    pub fn unapply(&self, external_force: &mut ExternalForce) {
        external_force.force -= self.force;
        external_force.torque -= self.torque;
    }

    pub fn reset(&mut self) {
        self.force = Vec3::ZERO;
        self.torque = Vec3::ZERO;
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 200.0,
                height: 200.0,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        //.add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_lighting)
        .add_startup_system(setup_objects)
        .add_system(add_random_force)
        .add_system(apply_transient_forces.at_end())
        .add_system_to_stage(
            PhysicsStages::SyncBackend,
            unapply_transient_forces.at_end(),
        )
        .run();
}

const BOX_SIZE: f32 = 2.0;
const BALL_SIZE: f32 = 0.5;
const BALL_COUNT: i32 = 5;
const BALL_FORCE: f32 = 5000.0;

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(BOX_SIZE * 2.0, BOX_SIZE * 1.5, BOX_SIZE * 2.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        FlyCam,
    ));
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            ..Default::default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::default(),
            0.0,
            -PI / 4.0,
            -PI / 4.0,
        )),
        ..Default::default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.1,
    });
}

fn setup_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wall_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::splat(BOX_SIZE * 2.0))));
    let wall_material = materials.add(Color::rgb(0.8, 0.8, 0.8).into());

    let faces = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::NEG_X,
        Vec3::NEG_Y,
        Vec3::NEG_Z,
    ];
    for face in faces.iter() {
        commands.spawn((
            PbrBundle {
                mesh: wall_mesh.clone(),
                material: wall_material.clone(),
                transform: Transform::from_xyz(
                    face.x * BOX_SIZE,
                    face.y * BOX_SIZE,
                    face.z * BOX_SIZE,
                )
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, *face)),
                ..Default::default()
            },
            Collider::cuboid(BOX_SIZE, BOX_SIZE, 0.1),
            Restitution::coefficient(0.8),
        ));
    }

    let ball_mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: BALL_SIZE,
        subdivisions: 1,
    }));

    for i in 0..BALL_COUNT {
        let hue = i as f32 / BALL_COUNT as f32 * 360.0;
        let color = Color::hsl(hue, 1.0, 1.0);
        commands.spawn((
            Player,
            PbrBundle {
                mesh: ball_mesh.clone(),
                material: materials.add(color.into()),
                transform: random_position(),
                ..Default::default()
            },
            TransientForce::default(),
            RigidBody::Dynamic,
            Collider::ball(BALL_SIZE),
            Ccd::enabled(),
            Restitution::coefficient(1.0),
            ExternalForce::default(),
        ));
    }
}

fn random_position() -> Transform {
    let mut rng = rand::thread_rng();
    let bound = BOX_SIZE - BALL_SIZE;
    let x = rng.gen_range(-bound..bound);
    let y = rng.gen_range(-bound..bound);
    let z = rng.gen_range(-bound..bound);
    Transform::from_xyz(x, y, z)
}

fn random_point_on_sphere() -> Vec3 {
    let mut rng = rand::thread_rng();
    let theta = rng.gen_range(0.0..PI);
    let phi = rng.gen_range(0.0..PI);
    let r = rng.gen_range(0.0..1.0);
    let x = r * theta.cos() * phi.cos();
    let y = r * theta.sin() * phi.cos();
    let z = r * phi.sin();
    Vec3::new(x, y, z)
}

fn add_random_force(
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut TransientForce, With<Player>>,
) {
    if input.just_pressed(KeyCode::Z) {
        for mut transient_force in query.iter_mut() {
            transient_force.force = random_point_on_sphere() * BALL_FORCE;
        }
    }
}

fn apply_transient_forces(mut query: Query<(&mut ExternalForce, &TransientForce)>) {
    for (mut external_force, transient_force) in query.iter_mut() {
        transient_force.apply(&mut external_force);
    }
}

fn unapply_transient_forces(mut query: Query<(&mut ExternalForce, &mut TransientForce)>) {
    for (mut external_force, mut transient_force) in query.iter_mut() {
        transient_force.unapply(&mut external_force);
        transient_force.reset();
    }
}
