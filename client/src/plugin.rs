use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use shared::{Request, Response};
use url::Url;

use crate::{client::PhysicsClient, error::Result, systems};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    Writeback,
}

pub struct RapierPhysicsPlugin {
    addr: String,
    port: u16,
}

impl RapierPhysicsPlugin {
    pub fn new() -> Self {
        Self {
            addr: "localhost".to_string(),
            port: 8080,
        }
    }

    pub fn with_addr(mut self, addr: &str) -> Self {
        self.addr = addr.to_string();
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

#[derive(Resource)]
pub struct PhysicsClientWrapper(pub Arc<Mutex<PhysicsClient>>);

// Couldn't get futures working with Bevy
// TODO: Implement this with futures instead of polling
#[cfg(feature = "bulk-requests")]
#[derive(Resource)]
pub struct RequestResult(pub Arc<Mutex<Option<Result<Response>>>>);

#[cfg(feature = "bulk-requests")]
impl Default for RequestResult {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

#[cfg(not(feature = "bulk-requests"))]
#[derive(Resource)]
pub struct RequestResult(pub Arc<Mutex<Vec<Result<Response>>>>);

#[cfg(not(feature = "bulk-requests"))]
impl Default for RequestResult {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut App) {
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

        app.insert_resource(RequestQueue::default());
        app.insert_resource(RequestResult::default());

        // Custom initialization

        app.add_stage_after(
            CoreStage::PreUpdate,
            PhysicsStage::SyncBackend,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_system(systems::update_config)
                    .with_system(systems::init_rigid_bodies.after(systems::update_config))
                    .with_system(systems::init_colliders.after(systems::init_rigid_bodies))
                    .with_system(systems::simulate_step.after(systems::init_colliders))
                    .with_system(systems::process_requests.after(systems::simulate_step)),
            ),
        );

        app.add_stage_before(
            PhysicsStage::SyncBackend,
            PhysicsStage::Writeback,
            SystemStage::parallel().with_system(systems::writeback), //with_run_criteria(FixedTimestep::steps_per_second(1.0))
        );

        let url = Url::parse(format!("ws://{}:{}/socket", self.addr, self.port).as_str()).unwrap();
        let client = PhysicsClient::new(url);
        let wrapper = PhysicsClientWrapper(Arc::new(Mutex::new(client)));
        app.insert_resource(wrapper);
    }
}

#[derive(Resource, Default)]

pub struct RequestQueue(pub Vec<Request>);
