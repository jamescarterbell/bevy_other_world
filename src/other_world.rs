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
    pub(crate) fn new() -> Self{
        Self{
            world: World::default()
        }
    }
}

#[cfg(test)]
mod tests{
    use core::ops::DerefMut;
    use core::ops::Deref;
    use crate::other_query::OtherQuery;
    use bevy::ecs::world::World;
    use bevy::winit::WinitConfig;
    use bevy::app::App;
    use bevy::prelude::*;

    #[test]
    fn simple_query(){
        let mut world = World::default();
        world
            .spawn()
            .insert(20u32);
        world
            .spawn()
            .insert(20u32);
        world
            .spawn()
                    .insert(20u32);
        App::build()
            .insert_resource(WinitConfig{
                return_from_run: true
            })
            .insert_resource(SubWorld{world})
            .add_system(other_query_test.system())
            .run();
    }

    fn other_query_test(q: OtherQuery<SubWorld, &u32>){
        println!("Running system!");
        for n in q.iter(){
            println!("{}", n);
        }
    }

    struct SubWorld{
        world: World,
    }

    impl Deref for SubWorld{
        type Target = World;

        fn deref(&self) -> &<Self as std::ops::Deref>::Target { &self.world }
    }

    impl DerefMut for SubWorld{
        fn deref_mut(&mut self) -> &mut <Self as std::ops::Deref>::Target { &mut self.world }
    }
}