use std::collections::HashMap;

use bevy::prelude::*;
use bevy_rapier3d::{
    prelude::*,
    rapier::prelude::{ColliderHandle, Isometry, RigidBodyHandle},
};

use serde::{Deserialize, Serialize};

pub mod serializable;
use serializable::*;

#[derive(Serialize, Deserialize)]
pub struct CreatedBody {
    pub id: u64,
    pub body: RigidBody,
    pub transform: Option<Isometry<Real>>,
    pub additional_mass_properties: Option<SerializableAdditionalMassProperties>,
}

#[derive(Serialize, Deserialize)]
pub struct CreatedCollider {
    pub id: u64,
    pub shape: Collider,
    pub transform: Option<Isometry<Real>>,
    pub sensor: Option<SerializableSensor>,
    pub mass_properties: Option<SerializableColliderMassProperties>,
    pub friction: Option<SerializableFriction>,
    pub restitution: Option<SerializableRestitution>,
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    UpdateConfig(SerializableRapierConfiguration),
    CreateBodies(Vec<CreatedBody>),
    CreateColliders(Vec<CreatedCollider>),
    SimulateStep(f32),
}

impl Request {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UpdateConfig(_) => "UpdateConfig",
            Self::CreateBodies(_) => "CreateBodies",
            Self::CreateColliders(_) => "CreateColliders",
            Self::SimulateStep(_) => "SimulateStep",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    ConfigUpdated,
    RigidBodyHandles(Vec<(u64, RigidBodyHandle)>),
    ColliderHandles(Vec<(u64, ColliderHandle)>),
    SimulationResult(HashMap<RigidBodyHandle, (Transform, Velocity)>),
}

pub fn transform_to_iso(transform: &Transform, physics_scale: Real) -> Isometry<Real> {
    Isometry::from_parts(
        (transform.translation / physics_scale).into(),
        transform.rotation.into(),
    )
}
