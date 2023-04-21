use physics_shared::*;
use tungstenite::handshake::server;
use tungstenite::http::StatusCode;

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;

use bevy_ecs::prelude::*;
use bevy_rapier3d::rapier::prelude::*;
use bevy_rapier3d::{prelude::*, utils};
use bevy_transform::prelude::*;

use bincode::{deserialize, serialize};
use clap::{arg, command, value_parser};
use rand::{thread_rng, Rng};
use tungstenite::{accept_hdr, Message};

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
                    clap::ErrorKind::ValueValidation,
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

    let mut is_socket_path = false;

    let websocket = accept_hdr(
        stream.try_clone()?,
        |req: &server::Request, mut response: server::Response| {
            println!("Received a new ws handshake");
            println!("The request's path is: {}", req.uri().path());
            println!("The request's headers are:");
            for (ref header, _value) in req.headers() {
                println!("* {}", header);
            }
            is_socket_path = req.uri().path() == "/ws";
            if is_socket_path {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }

            println!("Sending a handshake");
            println!("The response's status is: {}", response.status());
            println!("The response's headers are:");
            for (ref header, _value) in response.headers() {
                println!("* {}", header);
            }

            Ok(response)
        },
    );

    if let Err(e) = websocket {
        if let tungstenite::HandshakeError::Interrupted(_) = e {
            println!("Connection closed by {}", peer_addr);
            return Ok(());
        }

        if is_socket_path {
            let response = server::Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body("{\"error\": \"Access via websocket protocol\"}")
                .unwrap();
            server::write_response(stream, &response)?;
            return Ok(());
        }

        let response = server::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("{\"error\": \"Not found\"}")
            .unwrap();
        server::write_response(stream, &response)?;
        return Ok(());
    }

    let mut websocket = websocket.unwrap();

    println!("Connection from {}", peer_addr);

    let mut context = RapierContext::default();

    // dummy physics hooks
    #[allow(clippy::let_unit_value)]
    let physics_hooks = ();

    loop {
        println!("Waiting for message...");
        let msg = websocket.read_message()?;
        println!("Received message of length {:?}", msg.len());
        if msg.is_binary() {
            let req: Request = deserialize(&msg.into_data())?;
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

            simulate_latency(simulated_latency);

            websocket.write_message(Message::binary(bytes))?;
        } else if msg.is_close() {
            println!("Closing connection with {}", peer_addr);
            return Ok(());
        } else {
            return Err(format!("Unexpected message: {:?}", msg).into());
        }
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

fn create_bodies(bodies: Vec<CreatedBody>, context: &mut RapierContext) -> Response {
    println!("Creating bodies");
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
    println!("Creating colliders");
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
    println!("Simulating step");
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