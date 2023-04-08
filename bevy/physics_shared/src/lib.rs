use std::collections::HashMap;

use bevy_rapier3d::{
    prelude::*,
    rapier::prelude::{ColliderHandle, Isometry, RigidBodyHandle},
};
use bevy_time::prelude::Time;
use bevy_transform::prelude::Transform;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatedBody {
    pub id: u64,
    pub body: RigidBody,
    pub transform: Option<Isometry<Real>>,
    pub additional_mass_properties: Option<AdditionalMassProperties>,
}

#[derive(Serialize, Deserialize)]
pub struct CreatedCollider {
    pub id: u64,
    pub shape: Collider,
    pub transform: Option<Isometry<Real>>,
    pub sensor: Option<Sensor>,
    pub mass_properties: Option<ColliderMassProperties>,
    pub friction: Option<Friction>,
    pub restitution: Option<Restitution>,
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    CreateBodies(Vec<CreatedBody>),
    CreateColliders(Vec<CreatedCollider>),

    SimulateStep(Vect, TimestepMode, Time, SimulationToRenderTime),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    RigidBodyHandles(Vec<(u64, RigidBodyHandle)>),
    ColliderHandles(Vec<(u64, ColliderHandle)>),
    SimulationResult(HashMap<RigidBodyHandle, (Transform, Velocity)>),
}
