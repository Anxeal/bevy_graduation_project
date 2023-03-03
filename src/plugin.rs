use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub struct PhysicsWorldPlugin;

impl Plugin for PhysicsWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(
            RapierPhysicsPlugin::<NoUserData>::default().with_default_system_setup(false),
        );

        let physics_app = App::empty();

        physics_app.add_plugin(
            RapierPhysicsPlugin::<NoUserData>::default().with_default_system_setup(false),
        );

        physics_app.add_stage_after(
            CoreStage::Update,
            PhysicsStages::StepSimulation,
            SystemStage::parallel()
            .with_system_set(RapierPhysicsPlugin::get_systems(PhysicsStages::StepSimulation)),
        );

        app.add_stage_after(
            CoreStage::Update,
            PhysicsStages::SyncBackend,
            SystemStage::parallel()
                .with_system_set(RapierPhysicsPlugin::get_systems(PhysicsStages::SyncBackend)),
        );

        app.add_stage_after(
            PhysicsStages::SyncBackend,
            PhysicsStages::StepSimulation,
            SystemStage::parallel().with_system(update),
        );

        app.add_stage_after(
            PhysicsStages::StepSimulation,
            PhysicsStages::Writeback,
            SystemStage::parallel()
                .with_system_set(RapierPhysicsPlugin::get_systems(PhysicsStages::Writeback)),
        );

        // NOTE: we run sync_removals at the end of the frame, too, in order to make sure we don’t miss any `RemovedComponents`.
        app.add_stage_before(
            CoreStage::Last,
            PhysicsStages::DetectDespawn,
            SystemStage::parallel()
                .with_system_set(RapierPhysicsPlugin::get_systems(PhysicsStages::DetectDespawn)),
        );
    }
}
