use std::collections::HashMap;

use bevy::prelude::*;
use bevy_rapier3d::{
    prelude::*,
    rapier::prelude::{ColliderHandle, Isometry, RigidBodyHandle},
};

use serde::{Deserialize, Serialize};

pub mod serializable;
use serializable::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedBody {
    pub id: u64,
    pub body: RigidBody,
    pub transform: Option<Isometry<Real>>,
    pub additional_mass_properties: Option<SerializableAdditionalMassProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedCollider {
    pub id: u64,
    pub shape: Collider,
    pub transform: Option<Isometry<Real>>,
    pub sensor: Option<SerializableSensor>,
    pub mass_properties: Option<SerializableColliderMassProperties>,
    pub friction: Option<SerializableFriction>,
    pub restitution: Option<SerializableRestitution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    BulkRequest(Vec<Request>),
    UpdateConfig(SerializableRapierConfiguration),
    CreateBodies(Vec<CreatedBody>),
    CreateColliders(Vec<CreatedCollider>),
    SimulateStep(f32),
}

impl Request {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BulkRequest(_) => "BulkRequest",
            Self::UpdateConfig(_) => "UpdateConfig",
            Self::CreateBodies(_) => "CreateBodies",
            Self::CreateColliders(_) => "CreateColliders",
            Self::SimulateStep(_) => "SimulateStep",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    BulkResponse(Vec<Response>),
    ConfigUpdated,
    RigidBodyHandles(Vec<(u64, RigidBodyHandle)>),
    ColliderHandles(Vec<(u64, ColliderHandle)>),
    SimulationResult(HashMap<RigidBodyHandle, (Transform, Velocity)>),
}

impl Response {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BulkResponse(_) => "BulkResponse",
            Self::ConfigUpdated => "ConfigUpdated",
            Self::RigidBodyHandles(_) => "RigidBodyHandles",
            Self::ColliderHandles(_) => "ColliderHandles",
            Self::SimulationResult(_) => "SimulationResult",
        }
    }
}

pub fn transform_to_iso(transform: &Transform, physics_scale: Real) -> Isometry<Real> {
    Isometry::from_parts(
        (transform.translation / physics_scale).into(),
        transform.rotation.into(),
    )
}
