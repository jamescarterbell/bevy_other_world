use core::ops::DerefMut;
use bevy::app::AppBuilder;
use std::ops::Deref;
use bevy::app::Plugin;
use bevy::ecs::prelude::*;

pub struct OtherWorld<const N: usize>{
    world: World,
}

impl<const N: usize> Deref for OtherWorld<N>{
    type Target = World; 

    fn deref(&self) -> &<Self as std::ops::Deref>::Target { 
        &self.world
    }
}

impl<const N: usize> DerefMut for OtherWorld<N>{
    fn deref_mut(&mut self) -> &mut <Self as std::ops::Deref>::Target {
        &mut self.world
    }
}

impl<const N: usize> OtherWorld<N>{
    fn new() -> Self{
        Self{
            world: World::default()
        }
    }
}