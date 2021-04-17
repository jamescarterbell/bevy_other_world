use bevy::app::AppBuilder;
use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use crate::other_world::OtherWorld;
//use crate::other_query::OtherQuery;
use crate::other_world_query::{Other};
//use crate::other_commands::OtherCommands;

struct OtherWorldPlugin<const N: usize>{
    sync_stage: Option<Box<dyn StageSystemAdder>>,
    
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

struct Nonsynced;

fn sync_other_world<const N: usize>(nonsynced: Query<Other<(), N>, ()>){
    // First, despawn any sync entities in the outer world that don't exist in the inner world.
    // Then, spawn new sync entities for the Nonsynced entities in the inner world. (Remove Nonsynced Components as you go)
    // The Nonsynced components will never be seen by a rollback library as long as it does serializes the world, then runs it's schedule
    // (which is what it should do anyway to account for state 0)
}