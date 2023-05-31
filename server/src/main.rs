use bevy::prelude::*;
use bevy_rapier3d::rapier::prelude::{ColliderBuilder, RigidBodyBuilder, RigidBodyHandle};
use bevy_rapier3d::{prelude::*, utils};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::{Duration, Instant};

use bincode::{deserialize, serialize};
use clap::{arg, command, value_parser};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use rand::{thread_rng, Rng};
use tungstenite::{accept, Message};

use shared::*;

#[derive(Debug, Clone, Copy)]
enum SimulatedLatency {
    None,
    Fixed(u64),
    Random { min: u64, mean: u64 },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = command!()
        .arg(
            arg!(
                -p --port <PORT> "The port to listen on"
            )
            .required(false)
            .default_value("8080")
            .value_parser(value_parser!(u16).range(1..=65535)),
        )
        .arg(
            arg!(
                -l --latency <LATENCY> "The simulated latency in milliseconds, mean latency if min is specified"
            )
            .required(false)
            .value_parser(value_parser!(u64)),
        )
        .arg(
            arg!(
                -m --min <MIN> "The minimum simulated latency in milliseconds"
            )
            .required(false)
            .requires("latency")
            .value_parser(value_parser!(u64)),
        );

    let matches = cmd.get_matches_mut();

    let simulated_latency = match (
        matches.get_one::<u64>("latency"),
        matches.get_one::<u64>("min"),
    ) {
        (Some(&latency), None) => SimulatedLatency::Fixed(latency),
        (Some(&latency), Some(&min)) => {
            if min >= latency {
                cmd.error(
                    clap::error::ErrorKind::ValueValidation,
                    "min must be less than latency",
                );
            }
            SimulatedLatency::Random { min, mean: latency }
        }
        (None, None) => SimulatedLatency::None,
        _ => unreachable!(),
    };

    let port = matches.get_one::<u16>("port").unwrap();
    let server = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Listening on port {}", port);

    for stream in server.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, simulated_latency) {
                        println!("Error: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(
    stream: TcpStream,
    simulated_latency: SimulatedLatency,
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr()?;

    let mut websocket = accept(stream)?;

    println!("Connection from {}", peer_addr);

    let mut context = RapierContext::default();
    let mut config: Option<RapierConfiguration> = None;
    let mut sim_to_render_time = SimulationToRenderTime::default();
    let mut entity2body = HashMap::new();

    // dummy physics hooks
    #[allow(clippy::let_unit_value)]
    let physics_hooks = ();

    loop {
        println!("Waiting for message...");
        let msg = websocket.read_message()?;
        println!("Received message of length {:?}", msg.len());
        if msg.is_binary() {
            let msg_data = msg.into_data();

            let req = {
                #[cfg(feature = "compression")]
                {
                    let mut decoder = ZlibDecoder::new(msg_data.as_slice());
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed)?;

                    deserialize(&decompressed)?
                }
                #[cfg(not(feature = "compression"))]
                {
                    deserialize(&msg_data)?
                }
            };

            let response = handle_request(
                req,
                &mut context,
                &mut config,
                &mut sim_to_render_time,
                &mut entity2body,
                physics_hooks,
            );

            simulate_latency(simulated_latency);

            let serialized = serialize(&response)?;
            let msg = {
                #[cfg(feature = "compression")]
                {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&serialized)?;
                    let compressed = encoder.finish()?;
                    Message::binary(compressed)
                }
                #[cfg(not(feature = "compression"))]
                {
                    Message::binary(serialized)
                }
            };
            websocket.write_message(msg)?;
        } else if msg.is_close() {
            println!("Closing connection with {}", peer_addr);
            return Ok(());
        } else {
            return Err(format!("Unexpected message: {:?}", msg).into());
        }
    }
}

fn handle_request(
    req: Request,
    mut context: &mut RapierContext,
    mut config: &mut Option<RapierConfiguration>,
    mut sim_to_render_time: &mut SimulationToRenderTime,
    mut entity2body: &mut HashMap<Entity, RigidBodyHandle>,
    physics_hooks: (),
) -> Response {
    match req {
        Request::BulkRequest(reqs) => {
            let mut responses = vec![];
            for req in reqs {
                responses.push(handle_request(
                    req,
                    &mut context,
                    &mut config,
                    &mut sim_to_render_time,
                    &mut entity2body,
                    physics_hooks,
                ));
            }
            Response::BulkResponse(responses)
        }
        Request::UpdateConfig(new_config) => update_config(new_config.into(), &mut config),
        Request::CreateBodies(bodies) => create_bodies(bodies, &mut context, &mut entity2body),
        Request::CreateColliders(colliders) => {
            create_colliders(colliders, &mut context, &entity2body)
        }
        Request::SimulateStep(delta_time) => simulate_step(
            &mut context,
            config.unwrap().gravity,
            config.unwrap().timestep_mode,
            physics_hooks,
            delta_time,
            &mut sim_to_render_time,
        ),
    }
}

fn simulate_latency(simulated_latency: SimulatedLatency) {
    let latency = match simulated_latency {
        SimulatedLatency::None => return,
        SimulatedLatency::Fixed(latency) => latency,
        SimulatedLatency::Random { min, mean } => {
            let mut rng = thread_rng();
            let expovariate = -rng.gen::<f64>().ln() * (mean - min) as f64;
            (min as f64 + expovariate) as u64
        }
    };

    let latency = Duration::from_millis(latency);
    println!("Simulated Latency: {:?}", latency);
    sleep(latency);
}

fn update_config(
    new_config: RapierConfiguration,
    config: &mut Option<RapierConfiguration>,
) -> Response {
    *config = Some(new_config);
    Response::ConfigUpdated
}

fn create_bodies(
    bodies: Vec<CreatedBody>,
    context: &mut RapierContext,
    entity2body: &mut HashMap<Entity, RigidBodyHandle>,
) -> Response {
    println!("Creating bodies");
    let mut rbs = vec![];
    for body in bodies {
        let mut builder = RigidBodyBuilder::new(body.body.into());

        if let Some(transform) = body.transform {
            builder = builder.position(transform);
        }

        if let Some(mprops) = body.additional_mass_properties {
            builder = match mprops.into() {
                AdditionalMassProperties::MassProperties(mprops) => {
                    builder.additional_mass_properties(mprops.into_rapier(context.physics_scale()))
                }
                AdditionalMassProperties::Mass(mass) => builder.additional_mass(mass),
            };
        }

        builder = builder.user_data(body.id.into());

        let handle = context.bodies.insert(builder);

        entity2body.insert(Entity::from_bits(body.id), handle);

        rbs.push((body.id, handle));
    }
    Response::RigidBodyHandles(rbs)
}

fn create_colliders(
    colliders: Vec<CreatedCollider>,
    context: &mut RapierContext,
    entity2body: &HashMap<Entity, RigidBodyHandle>,
) -> Response {
    println!("Creating colliders");
    let mut cols = vec![];
    for collider in colliders {
        let mut builder = ColliderBuilder::new(collider.shape.raw);

        if let Some(mprops) = collider.mass_properties {
            builder = match mprops.into() {
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
        let body_handle = entity2body.get(&body_entity).copied();
        let child_transform = Transform::default();

        builder = builder.user_data(collider.id.into());

        let handle = if let Some(body_handle) = body_handle {
            builder = builder.position(transform_to_iso(&child_transform, context.physics_scale()));
            context
                .colliders
                .insert_with_parent(builder, body_handle, &mut context.bodies)
        } else {
            let transform = collider.transform.unwrap_or_default();
            builder = builder.position(transform);
            context.colliders.insert(builder)
        };

        // entity2collider.insert(Entity::from_bits(collider.id), handle);

        cols.push((collider.id, handle));
    }
    Response::ColliderHandles(cols)
}

fn simulate_step(
    context: &mut RapierContext,
    gravity: Vect,
    timestep_mode: TimestepMode,
    physics_hooks: (),
    delta_time: f32,
    sim_to_render_time: &mut SimulationToRenderTime,
) -> Response {
    println!("Simulating step");

    // Hack to get delta time into rapier
    let now = Instant::now();
    let then = now - Duration::from_secs_f32(delta_time);
    let mut time = Time::new(then);
    time.update_with_instant(then);
    time.update_with_instant(now);

    context.step_simulation(
        gravity,
        timestep_mode,
        None,
        &physics_hooks,
        &time,
        sim_to_render_time,
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
