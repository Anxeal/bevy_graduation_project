use std::collections::HashMap;

use bevy_time::Time;
use bevy_transform::prelude::Transform;
use crossbeam::channel::{bounded, Receiver, Sender};

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    prelude::Entity,
    schedule::{IntoSystemDescriptor, StageLabel, SystemSet, SystemStage},
    system::Resource,
};
use bevy_rapier3d::{
    prelude::*,
    rapier::prelude::{
        ColliderBuilder, ColliderHandle, Isometry, RigidBodyBuilder, RigidBodyHandle,
    },
    utils,
};

use crate::systems;

#[derive(Resource, Default)]
struct ScratchRapier(RapierContext, RapierConfiguration);

pub struct CreatedBody {
    pub id: u64,
    pub body: RigidBody,
    pub transform: Option<Isometry<Real>>,
    pub additional_mass_properties: Option<AdditionalMassProperties>,
}

pub struct CreatedCollider {
    pub id: u64,
    pub shape: Collider,
    pub transform: Option<Isometry<Real>>,
    pub sensor: Option<Sensor>,
    pub mass_properties: Option<ColliderMassProperties>,
    pub friction: Option<Friction>,
    pub restitution: Option<Restitution>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    Writeback,
}

pub enum Request {
    CreateBodies(Vec<CreatedBody>),
    CreateColliders(Vec<CreatedCollider>),
    SimulateStep(Vect, TimestepMode, Time, SimulationToRenderTime),
}

pub enum Response {
    RigidBodyHandles(Vec<(u64, RigidBodyHandle)>),
    ColliderHandles(Vec<(u64, ColliderHandle)>),
    SimulationResult(HashMap<RigidBodyHandle, (Transform, Velocity)>),
}

#[derive(Resource)]
pub struct RequestSender(pub Sender<Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<Response>);

pub struct RapierPhysicsPlugin;

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Default initialization
        // Register components as reflectable.
        app.register_type::<RigidBody>()
            .register_type::<Velocity>()
            .register_type::<AdditionalMassProperties>()
            .register_type::<MassProperties>()
            .register_type::<LockedAxes>()
            .register_type::<ExternalForce>()
            .register_type::<ExternalImpulse>()
            .register_type::<Sleeping>()
            .register_type::<Damping>()
            .register_type::<Dominance>()
            .register_type::<Ccd>()
            .register_type::<GravityScale>()
            .register_type::<CollidingEntities>()
            .register_type::<Sensor>()
            .register_type::<Friction>()
            .register_type::<Restitution>()
            .register_type::<CollisionGroups>()
            .register_type::<SolverGroups>()
            .register_type::<ContactForceEventThreshold>()
            .register_type::<Group>();

        // Insert all of our required resources. Don’t overwrite
        // the `RapierConfiguration` if it already exists.
        if app.world.get_resource::<RapierConfiguration>().is_none() {
            app.insert_resource(RapierConfiguration::default());
        }

        app.insert_resource(SimulationToRenderTime::default())
            .insert_resource(RapierContext::default());

        // Custom initialization

        app.add_stage_after(
            CoreStage::PostUpdate,
            PhysicsStage::SyncBackend,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_system(systems::init_rigid_bodies)
                    .with_system(systems::init_colliders.after(systems::init_rigid_bodies)),
            ),
        );

        app.add_stage_after(
            PhysicsStage::SyncBackend,
            PhysicsStage::Writeback,
            SystemStage::parallel().with_system(systems::writeback), //with_run_criteria(FixedTimestep::steps_per_second(1.0))
        );

        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        app.insert_resource(ResponseReceiver(res_rx));
        app.insert_resource(RequestSender(req_tx));

        std::thread::spawn(move || {
            let mut context = RapierContext::default();

            // dummy physics hooks
            #[allow(clippy::let_unit_value)]
            let physics_hooks = ();

            while let Ok(req) = req_rx.recv() {
                match req {
                    Request::CreateBodies(bodies) => {
                        let mut rbs = vec![];

                        for body in bodies {
                            let mut builder = RigidBodyBuilder::new(body.body.into());

                            if let Some(transform) = body.transform {
                                builder = builder.position(transform);
                            }

                            if let Some(mprops) = body.additional_mass_properties {
                                builder = match mprops {
                                    AdditionalMassProperties::MassProperties(mprops) => builder
                                        .additional_mass_properties(
                                            mprops.into_rapier(context.physics_scale()),
                                        ),
                                    AdditionalMassProperties::Mass(mass) => {
                                        builder.additional_mass(mass)
                                    }
                                };
                            }

                            builder = builder.user_data(body.id.into());

                            let handle = context.bodies.insert(builder);

                            context
                                .entity2body
                                .insert(Entity::from_bits(body.id), handle);

                            rbs.push((body.id, handle));
                        }

                        res_tx.send(Response::RigidBodyHandles(rbs)).unwrap();
                    }
                    Request::CreateColliders(colliders) => {
                        let mut cols = vec![];

                        for collider in colliders {
                            let mut builder = ColliderBuilder::new(collider.shape.raw);

                            if let Some(mprops) = collider.mass_properties {
                                builder = match mprops {
                                    ColliderMassProperties::Density(density) => {
                                        builder.density(density)
                                    }
                                    ColliderMassProperties::Mass(mass) => builder.mass(mass),
                                    ColliderMassProperties::MassProperties(mprops) => builder
                                        .mass_properties(
                                            mprops.into_rapier(context.physics_scale()),
                                        ),
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
                                context.colliders.insert_with_parent(
                                    builder,
                                    body_handle,
                                    &mut context.bodies,
                                )
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

                        res_tx.send(Response::ColliderHandles(cols)).unwrap();
                    }
                    Request::SimulateStep(gravity, timestep_mode, time, mut sim_to_render_time) => {
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

                        res_tx.send(Response::SimulationResult(results)).unwrap();
                    }
                }
            }
        });
    }
}
