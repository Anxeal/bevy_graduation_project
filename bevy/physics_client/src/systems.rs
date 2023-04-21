use std::time::Instant;

use bevy_ecs::{
    prelude::Entity,
    query::Without,
    system::{Commands, Query, Res, ResMut},
};
use bevy_rapier3d::prelude::{
    systems::RigidBodyWritebackComponents, AdditionalMassProperties, Collider,
    ColliderMassProperties, Friction, RapierColliderHandle, RapierConfiguration, RapierContext,
    RapierRigidBodyHandle, Restitution, RigidBody, Sensor, SimulationToRenderTime, Velocity,
};
use bevy_time::Time;
use bevy_transform::prelude::GlobalTransform;

use crate::plugin::PhysicsSocket;
use bincode::{deserialize, serialize};
use human_bytes::human_bytes;
use physics_shared::*;
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
                bevy_rapier3d::utils::transform_to_iso(
                    &transform.compute_transform(),
                    physics_scale,
                )
            }),
            additional_mass_properties: additional_mass_properties.copied(),
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
                bevy_rapier3d::utils::transform_to_iso(
                    &transform.compute_transform(),
                    physics_scale,
                )
            }),
            sensor: sensor.copied(),
            mass_properties: mprops.copied(),
            friction: friction.copied(),
            restitution: restitution.copied(),
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
    let req = Request::SimulateStep(
        config.gravity,
        config.timestep_mode,
        time.clone(),
        SimulationToRenderTime {
            diff: sim_to_render_time.diff,
        },
    );

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
        "Sent {} and received {} in {:?}",
        human_bytes(sent_size as f64),
        human_bytes(msg.len() as f64),
        start.elapsed()
    );
    Ok(deserialize::<Response>(&msg.into_data())?)
}
