use std::time::Instant;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_rapier3d::plugin::systems::RigidBodyWritebackComponents;

use crate::plugin::PhysicsSocket;
use bincode::{deserialize, serialize};
use human_bytes::human_bytes;
use shared::*;
use tungstenite::Message;

pub type RigidBodyComponents<'a> = (
    Entity,
    &'a RigidBody,
    Option<&'a GlobalTransform>,
    Option<&'a Velocity>,
    Option<&'a AdditionalMassProperties>,
);

pub type ColliderComponents<'a> = (
    Entity,
    &'a Collider,
    Option<&'a Sensor>,
    Option<&'a ColliderMassProperties>,
    Option<&'a Friction>,
    Option<&'a Restitution>,
);

pub fn update_config(socket: ResMut<PhysicsSocket>, config: Res<RapierConfiguration>) {
    if !config.is_changed() {
        return;
    }

    let resp = send_request(socket, Request::UpdateConfig(config.clone().into()));

    if let Err(err) = resp {
        error!("Failed to update config: {}", err);
    } else if let Ok(Response::ConfigUpdated) = resp {
        info!("Config updated");
    } else {
        error!("Unexpected response");
    }
}

pub fn init_rigid_bodies(
    mut commands: Commands,
    context: Res<RapierContext>,
    socket: ResMut<PhysicsSocket>,
    rigid_bodies: Query<RigidBodyComponents, Without<RapierRigidBodyHandle>>,
) {
    let mut created_bodies = vec![];

    let physics_scale = context.physics_scale();

    for (entity, rb, transform, velocity, additional_mass_properties) in rigid_bodies.iter() {
        created_bodies.push(CreatedBody {
            id: entity.to_bits(),
            body: *rb,
            transform: transform.map(|transform| {
                shared::transform_to_iso(&transform.compute_transform(), physics_scale)
            }),
            additional_mass_properties: additional_mass_properties
                .map(|mprops| mprops.clone().into()),
        });
    }

    if created_bodies.is_empty() {
        return;
    }

    let resp = send_request(socket, Request::CreateBodies(created_bodies));

    if let Ok(Response::RigidBodyHandles(handles)) = resp {
        for handle in handles {
            commands
                .entity(Entity::from_bits(handle.0))
                .insert(RapierRigidBodyHandle(handle.1));
        }
    }
}

pub fn init_colliders(
    mut commands: Commands,
    context: Res<RapierContext>,
    socket: ResMut<PhysicsSocket>,
    colliders: Query<(ColliderComponents, Option<&GlobalTransform>), Without<RapierColliderHandle>>,
) {
    let mut created_colliders = vec![];

    let physics_scale = context.physics_scale();

    for ((entity, shape, sensor, mprops, friction, restitution), transform) in colliders.iter() {
        created_colliders.push(CreatedCollider {
            id: entity.to_bits(),
            shape: shape.clone(),
            transform: transform.map(|transform| {
                shared::transform_to_iso(&transform.compute_transform(), physics_scale)
            }),
            sensor: sensor.map(|sensor| sensor.clone().into()),
            mass_properties: mprops.map(|mprops| mprops.clone().into()),
            friction: friction.map(|friction| friction.clone().into()),
            restitution: restitution.map(|restitution| restitution.clone().into()),
        });
    }

    if created_colliders.is_empty() {
        return;
    }

    let resp = send_request(socket, Request::CreateColliders(created_colliders));

    if let Ok(Response::ColliderHandles(handles)) = resp {
        for handle in handles {
            commands
                .entity(Entity::from_bits(handle.0))
                .insert(RapierColliderHandle(handle.1));
        }
    }
}

pub fn writeback(
    context: Res<RapierContext>,
    config: Res<RapierConfiguration>,
    (time, sim_to_render_time): (Res<Time>, Res<SimulationToRenderTime>),
    socket: ResMut<PhysicsSocket>,
    mut rigid_bodies: Query<(RigidBodyWritebackComponents, &RapierRigidBodyHandle)>,
) {
    let req = Request::SimulateStep(time.delta_seconds());

    let resp = send_request(socket, req);

    if let Ok(Response::SimulationResult(result)) = resp {
        for ((entity, parent, transform, mut interpolation, mut velocity, mut sleeping), handle) in
            rigid_bodies.iter_mut()
        {
            let (new_transform, new_velocity) = result.get(&handle.0).unwrap();

            if let Some(mut transform) = transform {
                transform.translation = new_transform.translation;
                transform.rotation = new_transform.rotation;
            }

            if let Some(velocity) = &mut velocity {
                // NOTE: we write the new value only if there was an
                //       actual change, in order to not trigger bevy’s
                //       change tracking when the values didn’t change.
                if **velocity != *new_velocity {
                    **velocity = *new_velocity;
                }
            }
        }
    }
}

fn send_request(
    mut socket: ResMut<PhysicsSocket>,
    request: Request,
) -> Result<Response, Box<dyn std::error::Error>> {
    let msg = Message::Binary(serialize(&request)?);
    let sent_size = msg.len();

    let start = Instant::now();
    socket.0.write_message(msg.clone())?;

    let msg = socket.0.read_message()?;

    println!(
        "{}: Sent {} and received {} in {:?}",
        request.name(),
        human_bytes(sent_size as f64),
        human_bytes(msg.len() as f64),
        start.elapsed()
    );
    Ok(deserialize::<Response>(&msg.into_data())?)
}
