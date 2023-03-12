//! Copied from https://github.com/bevyengine/bevy/blob/main/examples/games/breakout.rs
//! A simplified implementation of the classic game "Breakout".

use std::f32::consts::PI;

use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets};
use bevy_core::{FrameCountPlugin, TaskPoolPlugin};
use bevy_core_pipeline::{prelude::Camera3dBundle, CorePipelinePlugin};
use bevy_ecs::{
    prelude::Component,
    query::With,
    system::{Commands, Query, Res, ResMut},
};
use bevy_log::LogPlugin;
use bevy_math::{Quat, Vec3};
use bevy_pbr::PbrPlugin;
use bevy_pbr::{PbrBundle, PointLight, PointLightBundle, StandardMaterial};
use bevy_render::{
    color::Color,
    prelude::Mesh,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};
use bevy_time::{Time, TimePlugin};
use bevy_transform::{prelude::Transform, TransformPlugin};
use bevy_utils::prelude::default;
use bevy_window::WindowPlugin;
use bevy_winit::WinitPlugin;

mod input;
mod render;

fn main() {
    App::new()
        .add_plugin(LogPlugin::default())
        .add_plugin(TaskPoolPlugin::default())
        .add_plugin(FrameCountPlugin::default())
        .add_plugin(TimePlugin)
        .add_plugin(TransformPlugin)
        .add_plugin(input::InputPlugin)
        .add_plugin(WindowPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(WinitPlugin)
        .add_plugin(render::RenderPlugin)
        .add_plugin(CorePipelinePlugin)
        .add_plugin(PbrPlugin::default())
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(bevy_window::close_on_esc)
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 14.5;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let shapes = [
        meshes.add(bevy_render::prelude::shape::Cube::default().into()),
        meshes.add(bevy_render::prelude::shape::Box::default().into()),
        meshes.add(bevy_render::prelude::shape::Capsule::default().into()),
        meshes.add(bevy_render::prelude::shape::Torus::default().into()),
        meshes.add(bevy_render::prelude::shape::Cylinder::default().into()),
        meshes.add(
            bevy_render::prelude::shape::Icosphere::default()
                .try_into()
                .unwrap(),
        ),
        meshes.add(bevy_render::prelude::shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(bevy_render::prelude::shape::Plane { size: 50.0 }.into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    )
}
