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
        let mut app = App::build();
            app.insert_resource(WinitConfig{
                return_from_run: true
            })
            .add_startup_system(test_startup.system())
            .add_system(other_query_test.system())
            .add_system(normal_query_test.system());
        let mut app = app.app;
        app.update();
    }

    fn test_startup(mut commands: Commands){
        let mut world = World::default();
        world
            .spawn()
            .insert(20u32)
            .insert(-21i32);
        world
            .spawn()
            .insert(20u32);
        world
            .spawn()
            .insert(20u32);
        commands.insert_resource(SubWorld{world});
        commands
            .spawn()
            .insert(10u32);
        commands
            .spawn()
            .insert(10u32)
            .insert(-10i32);
        commands
            .spawn()
            .insert(10u32);
        commands
            .spawn()
            .insert(11u32)
            .insert(-11i32);
    }

    fn other_query_test(q: OtherQuery<SubWorld, (&u32, &i32)>){
        println!("Running system!");
        for (u, i) in q.iter(){
            println!("{}, {}", u, i);
        }
    }

    fn normal_query_test(q: Query<(&u32, &i32)>){
        std::thread::sleep_ms(100);
        println!("Running system!");
        for (u, i) in q.iter(){
            println!("{}, {}", u, i);
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