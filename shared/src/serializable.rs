use bevy_rapier3d::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableMassProperties {
    pub local_center_of_mass: Vect,
    pub mass: f32,
    pub principal_inertia_local_frame: bevy_rapier3d::math::Rot,
    pub principal_inertia: Vect,
}

impl From<MassProperties> for SerializableMassProperties {
    fn from(mass_properties: MassProperties) -> Self {
        Self {
            local_center_of_mass: mass_properties.local_center_of_mass,
            mass: mass_properties.mass,
            principal_inertia_local_frame: mass_properties.principal_inertia_local_frame,
            principal_inertia: mass_properties.principal_inertia,
        }
    }
}

impl From<SerializableMassProperties> for MassProperties {
    fn from(mass_properties: SerializableMassProperties) -> Self {
        Self {
            local_center_of_mass: mass_properties.local_center_of_mass,
            mass: mass_properties.mass,
            principal_inertia_local_frame: mass_properties.principal_inertia_local_frame,
            principal_inertia: mass_properties.principal_inertia,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableColliderMassProperties {
    Density(f32),
    Mass(f32),
    MassProperties(SerializableMassProperties),
}

impl From<ColliderMassProperties> for SerializableColliderMassProperties {
    fn from(mass_properties: ColliderMassProperties) -> Self {
        match mass_properties {
            ColliderMassProperties::Density(density) => Self::Density(density),
            ColliderMassProperties::Mass(mass) => Self::Mass(mass),
            ColliderMassProperties::MassProperties(mass_properties) => {
                Self::MassProperties(mass_properties.into())
            }
        }
    }
}

impl From<SerializableColliderMassProperties> for ColliderMassProperties {
    fn from(mass_properties: SerializableColliderMassProperties) -> Self {
        match mass_properties {
            SerializableColliderMassProperties::Density(density) => Self::Density(density),
            SerializableColliderMassProperties::Mass(mass) => Self::Mass(mass),
            SerializableColliderMassProperties::MassProperties(mass_properties) => {
                Self::MassProperties(mass_properties.into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableAdditionalMassProperties {
    Mass(f32),
    MassProperties(SerializableMassProperties),
}

impl From<AdditionalMassProperties> for SerializableAdditionalMassProperties {
    fn from(mass_properties: AdditionalMassProperties) -> Self {
        match mass_properties {
            AdditionalMassProperties::Mass(mass) => Self::Mass(mass),
            AdditionalMassProperties::MassProperties(mass_properties) => {
                Self::MassProperties(mass_properties.into())
            }
        }
    }
}

impl From<SerializableAdditionalMassProperties> for AdditionalMassProperties {
    fn from(mass_properties: SerializableAdditionalMassProperties) -> Self {
        match mass_properties {
            SerializableAdditionalMassProperties::Mass(mass) => Self::Mass(mass),
            SerializableAdditionalMassProperties::MassProperties(mass_properties) => {
                Self::MassProperties(mass_properties.into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSensor;

impl From<Sensor> for SerializableSensor {
    fn from(_: Sensor) -> Self {
        Self
    }
}

impl From<SerializableSensor> for Sensor {
    fn from(_: SerializableSensor) -> Self {
        Self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFriction {
    pub coefficient: f32,
    pub combine_rule: CoefficientCombineRule,
}

impl From<Friction> for SerializableFriction {
    fn from(friction: Friction) -> Self {
        Self {
            coefficient: friction.coefficient,
            combine_rule: friction.combine_rule,
        }
    }
}

impl From<SerializableFriction> for Friction {
    fn from(friction: SerializableFriction) -> Self {
        Self {
            coefficient: friction.coefficient,
            combine_rule: friction.combine_rule,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableRestitution {
    pub coefficient: f32,
    pub combine_rule: CoefficientCombineRule,
}

impl From<Restitution> for SerializableRestitution {
    fn from(restitution: Restitution) -> Self {
        Self {
            coefficient: restitution.coefficient,
            combine_rule: restitution.combine_rule,
        }
    }
}

impl From<SerializableRestitution> for Restitution {
    fn from(restitution: SerializableRestitution) -> Self {
        Self {
            coefficient: restitution.coefficient,
            combine_rule: restitution.combine_rule,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableTimestepMode {
    Fixed {
        dt: f32,
        substeps: usize,
    },
    Variable {
        max_dt: f32,
        time_scale: f32,
        substeps: usize,
    },
    Interpolated {
        dt: f32,
        time_scale: f32,
        substeps: usize,
    },
}

impl From<TimestepMode> for SerializableTimestepMode {
    fn from(mode: TimestepMode) -> Self {
        match mode {
            TimestepMode::Fixed { dt, substeps } => Self::Fixed { dt, substeps },
            TimestepMode::Variable {
                max_dt,
                time_scale,
                substeps,
            } => Self::Variable {
                max_dt,
                time_scale,
                substeps,
            },
            TimestepMode::Interpolated {
                dt,
                time_scale,
                substeps,
            } => Self::Interpolated {
                dt,
                time_scale,
                substeps,
            },
        }
    }
}

impl From<SerializableTimestepMode> for TimestepMode {
    fn from(mode: SerializableTimestepMode) -> Self {
        match mode {
            SerializableTimestepMode::Fixed { dt, substeps } => Self::Fixed { dt, substeps },
            SerializableTimestepMode::Variable {
                max_dt,
                time_scale,
                substeps,
            } => Self::Variable {
                max_dt,
                time_scale,
                substeps,
            },
            SerializableTimestepMode::Interpolated {
                dt,
                time_scale,
                substeps,
            } => Self::Interpolated {
                dt,
                time_scale,
                substeps,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableRapierConfiguration {
    pub gravity: Vect,
    pub physics_pipeline_active: bool,
    pub query_pipeline_active: bool,
    pub timestep_mode: SerializableTimestepMode,
    pub scaled_shape_subdivision: u32,
    pub force_update_from_transform_changes: bool,
}

impl From<RapierConfiguration> for SerializableRapierConfiguration {
    fn from(config: RapierConfiguration) -> Self {
        Self {
            gravity: config.gravity,
            physics_pipeline_active: config.physics_pipeline_active,
            query_pipeline_active: config.query_pipeline_active,
            timestep_mode: config.timestep_mode.into(),
            scaled_shape_subdivision: config.scaled_shape_subdivision,
            force_update_from_transform_changes: config.force_update_from_transform_changes,
        }
    }
}

impl From<SerializableRapierConfiguration> for RapierConfiguration {
    fn from(config: SerializableRapierConfiguration) -> Self {
        Self {
            gravity: config.gravity,
            physics_pipeline_active: config.physics_pipeline_active,
            query_pipeline_active: config.query_pipeline_active,
            timestep_mode: config.timestep_mode.into(),
            scaled_shape_subdivision: config.scaled_shape_subdivision,
            force_update_from_transform_changes: config.force_update_from_transform_changes,
        }
    }
}
