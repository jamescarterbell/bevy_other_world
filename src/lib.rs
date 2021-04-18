pub mod other_world;
pub mod plugin;
//pub mod other_commands;
//pub mod other_world_query;
//pub mod other_query_state;

#[cfg(test)]
mod tests{
    use bevy::winit::WinitConfig;
    use bevy::prelude::*;
    use crate::*;

    #[test]
    fn create_add(){
        App::build()
            .insert_resource(other_world::OtherWorld::<0>::new())
            .insert_resource(WinitConfig{
                return_from_run: true,
            })
            .run();
    }

    #[test]
    fn create_add_query(){
        App::build()
            .insert_resource(other_world::OtherWorld::<0>::new())
            .insert_resource(WinitConfig{
                return_from_run: true,
            })
            .add_startup_system(add_entities.system())
            .add_system(check_3.system())
            .run();
    }

    fn add_entities(mut other_world: ResMut<other_world::OtherWorld<0>>){
        other_world
            .spawn()
            .insert(3usize);
            let mut other_query = other_world.query::<&usize>();
            let mut count = 0;
            for num in other_query.iter(&mut other_world){
                assert_eq!(*num, 3usize);
                count += 1;
            }
            assert_eq!(count, 1);
    }

    fn check_3(other_query: Query<other_world_query::Other<&usize, 0>>){
        let mut count = 0;
        for num in other_query.iter(){
            assert_eq!(*num, 3usize);
            count += 1;
        }
        assert_eq!(count, 1);
    }
}