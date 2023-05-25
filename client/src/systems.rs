use std::thread;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_rapier3d::plugin::systems::RigidBodyWritebackComponents;

use crate::error::Result;
use crate::plugin::{
    PhysicsClientWrapper, RapierPhysicsPluginConfiguration, RequestQueue, RequestResult,
};
use shared::*;

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

pub fn update_config(config: Res<RapierConfiguration>, mut request_queue: ResMut<RequestQueue>) {
    if !config.is_changed() {
        return;
    }

    let req = Request::UpdateConfig(config.clone().into());

    request_queue.0.push(req);
}

fn handle_update_config_response(resp: Result<Response>) {
    if let Err(err) = resp {
        error!("Failed to update config: {}", err);
    } else if let Ok(Response::ConfigUpdated) = resp {
        info!("Config updated");
    } else {
        error!("Unexpected response");
    }
}

pub fn init_rigid_bodies(
    context: Res<RapierContext>,
    rigid_bodies: Query<RigidBodyComponents, Without<RapierRigidBodyHandle>>,
    mut request_queue: ResMut<RequestQueue>,
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

    request_queue.0.push(Request::CreateBodies(created_bodies));
}

fn handle_init_rigid_bodies_response(resp: Result<Response>, commands: &mut Commands) {
    if let Ok(Response::RigidBodyHandles(handles)) = resp {
        for handle in handles {
            commands
                .entity(Entity::from_bits(handle.0))
                .insert(RapierRigidBodyHandle(handle.1));
        }
    }
}

pub fn init_colliders(
    context: Res<RapierContext>,
    colliders: Query<(ColliderComponents, Option<&GlobalTransform>), Without<RapierColliderHandle>>,
    mut request_queue: ResMut<RequestQueue>,
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

    request_queue
        .0
        .push(Request::CreateColliders(created_colliders));
}

fn handle_init_colliders_response(resp: Result<Response>, commands: &mut Commands) {
    if let Ok(Response::ColliderHandles(handles)) = resp {
        for handle in handles {
            commands
                .entity(Entity::from_bits(handle.0))
                .insert(RapierColliderHandle(handle.1));
        }
    }
}

pub fn simulate_step(time: Res<Time>, mut request_queue: ResMut<RequestQueue>) {
    request_queue
        .0
        .push(Request::SimulateStep(time.delta_seconds()));
}

fn handle_simulate_step_response(
    resp: Result<Response>,
    rigid_bodies: &mut Query<(RigidBodyWritebackComponents, &RapierRigidBodyHandle)>,
) {
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

pub fn process_requests(
    mut request_queue: ResMut<RequestQueue>,
    client: Res<PhysicsClientWrapper>,
    result: Res<RequestResult>,
) {
    let req = Request::BulkRequest(request_queue.0.drain(..).collect());
    let client = client.0.clone();
    let result = result.0.clone();

    thread::spawn(move || {
        let resp = client.lock().unwrap().send_request(req);
        result.lock().unwrap().replace(resp);
    });
}

pub fn writeback(
    mut commands: Commands,
    mut rigid_bodies: Query<(RigidBodyWritebackComponents, &RapierRigidBodyHandle)>,
    result: Res<RequestResult>,
    mut init: Local<bool>,
) {
    if !*init {
        *init = true;
        return;
    }

    while result.0.lock().unwrap().is_none() {}
    let resp = result.0.lock().unwrap().take().unwrap();
    if let Err(err) = resp {
        error!("Failed to send request: {}", err);
        return;
    }

    if let Response::BulkResponse(responses) = resp.unwrap() {
        for resp in responses {
            match resp {
                Response::ConfigUpdated => {
                    handle_update_config_response(Ok(resp));
                }
                Response::RigidBodyHandles(_) => {
                    handle_init_rigid_bodies_response(Ok(resp), &mut commands);
                }
                Response::ColliderHandles(_) => {
                    handle_init_colliders_response(Ok(resp), &mut commands);
                }
                Response::SimulationResult(_) => {
                    handle_simulate_step_response(Ok(resp), &mut rigid_bodies);
                }
                _ => {
                    error!("Unexpected response");
                }
            }
        }
    } else {
        error!("Unexpected response");
    }
}
