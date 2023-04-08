use physics_shared::*;
use std::collections::HashMap;
use std::io::{Read, Write};

use bevy_ecs::prelude::*;
use bevy_rapier3d::rapier::prelude::*;
use bevy_rapier3d::{prelude::*, utils};
use bevy_transform::prelude::*;
use std::net::TcpListener;

use bincode::{deserialize, serialize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("localhost:8080").unwrap();

    println!("Listening on port 8080");

    let (mut stream, _) = listener.accept().unwrap();

    let mut buffer = [0; 1024];

    let mut context = RapierContext::default();

    // dummy physics hooks
    #[allow(clippy::let_unit_value)]
    let physics_hooks = ();

    loop {
        stream.read(&mut buffer)?;
        let Ok(req) = deserialize(&buffer) else { continue };
        let response = match req {
            Request::CreateBodies(bodies) => create_bodies(bodies, &mut context),
            Request::CreateColliders(colliders) => create_colliders(colliders, &mut context),
            Request::SimulateStep(gravity, timestep_mode, time, sim_to_render_time) => {
                simulate_step(
                    &mut context,
                    gravity,
                    timestep_mode,
                    physics_hooks,
                    time,
                    sim_to_render_time,
                )
            }
        };

        let bytes = serialize(&response)?;
        stream.write(&bytes)?;
    }
}

fn create_bodies(bodies: Vec<CreatedBody>, context: &mut RapierContext) -> Response {
    let mut rbs = vec![];
    for body in bodies {
        let mut builder = RigidBodyBuilder::new(body.body.into());

        if let Some(transform) = body.transform {
            builder = builder.position(transform);
        }

        if let Some(mprops) = body.additional_mass_properties {
            builder = match mprops {
                AdditionalMassProperties::MassProperties(mprops) => {
                    builder.additional_mass_properties(mprops.into_rapier(context.physics_scale()))
                }
                AdditionalMassProperties::Mass(mass) => builder.additional_mass(mass),
            };
        }

        builder = builder.user_data(body.id.into());

        let handle = context.bodies.insert(builder);

        context
            .entity2body
            .insert(Entity::from_bits(body.id), handle);

        rbs.push((body.id, handle));
    }
    Response::RigidBodyHandles(rbs)
}

fn create_colliders(colliders: Vec<CreatedCollider>, context: &mut RapierContext) -> Response {
    let mut cols = vec![];
    for collider in colliders {
        let mut builder = ColliderBuilder::new(collider.shape.raw);

        if let Some(mprops) = collider.mass_properties {
            builder = match mprops {
                ColliderMassProperties::Density(density) => builder.density(density),
                ColliderMassProperties::Mass(mass) => builder.mass(mass),
                ColliderMassProperties::MassProperties(mprops) => {
                    builder.mass_properties(mprops.into_rapier(context.physics_scale()))
                }
            };
        }

        if let Some(friction) = collider.friction {
            builder = builder
                .friction(friction.coefficient)
                .friction_combine_rule(friction.combine_rule.into());
        }

        if let Some(restitution) = collider.restitution {
            builder = builder
                .restitution(restitution.coefficient)
                .restitution_combine_rule(restitution.combine_rule.into());
        }

        let body_entity = Entity::from_bits(collider.id);
        let body_handle = context.entity2body.get(&body_entity).copied();
        let child_transform = Transform::default();

        builder = builder.user_data(collider.id.into());

        let handle = if let Some(body_handle) = body_handle {
            builder = builder.position(utils::transform_to_iso(
                &child_transform,
                context.physics_scale(),
            ));
            context
                .colliders
                .insert_with_parent(builder, body_handle, &mut context.bodies)
        } else {
            let transform = collider.transform.unwrap_or_default();
            builder = builder.position(transform);
            context.colliders.insert(builder)
        };

        context
            .entity2collider
            .insert(Entity::from_bits(collider.id), handle);

        cols.push((collider.id, handle));
    }
    Response::ColliderHandles(cols)
}

fn simulate_step(
    context: &mut RapierContext,
    gravity: Vect,
    timestep_mode: TimestepMode,
    physics_hooks: (),
    time: bevy_time::Time,
    mut sim_to_render_time: SimulationToRenderTime,
) -> Response {
    context.step_simulation(
        gravity,
        timestep_mode,
        None,
        &physics_hooks,
        &time,
        &mut sim_to_render_time,
        None,
    );

    let scale = context.physics_scale();

    let mut results = HashMap::new();

    for (handle, rb) in context.bodies.iter() {
        let transform = utils::iso_to_transform(rb.position(), scale);
        let velocity = Velocity {
            linvel: (rb.linvel() * scale).into(),
            angvel: (*rb.angvel()).into(),
        };

        results.insert(handle, (transform, velocity));
    }
    Response::SimulationResult(results)
}
