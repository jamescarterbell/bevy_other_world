use bevy::app::AppBuilder;
use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use crate::other_world::OtherWorld;
//use crate::other_query::OtherQuery;
//use crate::other_world_query::{Other};
//use crate::other_commands::OtherCommands;

pub struct OtherWorldPlugin<const N: usize>{
    sync_stage: Option<Box<dyn StageSystemAdder>>,
    
}

impl<const N: usize> OtherWorldPlugin<N>{
    pub fn new() -> Self{
        Self{
            sync_stage: None,
        }
    }

    pub fn with_sync<T: StageLabel + Clone>(&mut self, stage: T) -> &mut Self{
        self.sync_stage = Some(Box::new(SyncSystemAdder::<T, N>{
            stage
        }));
        self
    }
}

impl<const N: usize> Plugin for OtherWorldPlugin<N>{
    fn build(&self, app: &mut bevy::prelude::AppBuilder) { 
        app
            .insert_resource(OtherWorld::<N>::new());
        if let Some(ref stage) = self.sync_stage{
            stage.add_system(app);
        }
    }
}

pub trait StageSystemAdder: Send + Sync{
    fn add_system(&self, app: &mut AppBuilder);
}

struct SyncSystemAdder<T: StageLabel + Clone, const N: usize>{
    stage: T
}

unsafe impl<T: StageLabel + Clone, const N: usize> Send for SyncSystemAdder<T, N>{}
unsafe impl<T: StageLabel + Clone, const N: usize> Sync for SyncSystemAdder<T, N>{}

impl<T: StageLabel + Clone, const N: usize> StageSystemAdder for SyncSystemAdder<T, N>{
    fn add_system(&self, app: &mut AppBuilder){
        app.add_system_to_stage(self.stage.clone(), sync_other_world::<N>.system());
    }
}

pub struct Synced;
pub struct OtherSynced<const N: usize>{
    other: Entity,
}

pub fn sync_other_world<const N: usize>(mut commands: Commands, synced: Query<(&Entity, &OtherSynced<N>)>, mut other_world: ResMut<OtherWorld<N>>){
    // First, despawn any sync entities in the outer world that don't exist in the inner world.
    // TODO: Make this also check if the entity has changed (perhaps the inner world has been swapped in the other_world)
    synced
        .iter()
        .filter(|e| other_world.entities().contains(e.1.other))
        .for_each(|e|
            commands
                .entity(e.0.clone())
                .despawn()
        );
    // Then, spawn new sync entities for the Nonsynced entities in the inner world. (Remove Nonsynced Components as you go)
    let mut unsynced = other_world.query_filtered::<&Entity, Without<&Synced>>();
    let mut needs_addition = Vec::new();
    unsynced
        .for_each(&other_world, |e|{
            commands
                .spawn()
                .insert(OtherSynced::<N>{
                    other: e.clone()
                });
            needs_addition.push(e.clone());
        });
    
    needs_addition
        .drain(..)
        .for_each(|e|{
            other_world
                .entity_mut(e)
                .insert(Synced);
        });
}