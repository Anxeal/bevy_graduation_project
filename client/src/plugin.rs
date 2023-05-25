use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use shared::Request;
use url::Url;

use crate::{client::PhysicsClient, systems};

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

        if app
            .world
            .get_resource::<RapierPhysicsPluginConfiguration>()
            .is_none()
        {
            app.insert_resource(RapierPhysicsPluginConfiguration::default());
        }

        app.insert_resource(SimulationToRenderTime::default())
            .insert_resource(RapierContext::default());

        app.insert_resource(RequestQueue::default());

        // Custom initialization

        app.add_stage_after(
            CoreStage::PostUpdate,
            PhysicsStage::SyncBackend,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_system(systems::update_config)
                    .with_system(systems::init_rigid_bodies.after(systems::update_config))
                    .with_system(systems::init_colliders.after(systems::init_rigid_bodies))
                    .with_system(systems::simulate_step.after(systems::init_colliders)),
            ),
        );

        app.add_stage_after(
            PhysicsStage::SyncBackend,
            PhysicsStage::Writeback,
            SystemStage::parallel().with_system(systems::process_requests), //with_run_criteria(FixedTimestep::steps_per_second(1.0))
        );

        let url = Url::parse(format!("ws://{}:{}/socket", self.addr, self.port).as_str()).unwrap();
        app.insert_resource(PhysicsClient::new(url));
    }
}

#[derive(Resource)]
pub struct RapierPhysicsPluginConfiguration {
    pub bulk_requests: bool,
    pub compression: bool,
}

impl Default for RapierPhysicsPluginConfiguration {
    fn default() -> Self {
        Self {
            bulk_requests: true,
            compression: true,
        }
    }
}

#[derive(Resource)]

pub struct RequestQueue(pub Vec<Request>);

impl Default for RequestQueue {
    fn default() -> Self {
        Self(vec![])
    }
}
