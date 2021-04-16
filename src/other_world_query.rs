use crate::other_world::OtherWorld;
use bevy::ecs::world::World;
use bevy::ecs::query::ReadOnlyFetch;
use bevy::ecs::query::ReadState;
use bevy::ecs::query::ReadFetch;
use bevy::ecs::component::Component;
use bevy::ecs::query::FetchState;
use bevy::ecs::query::Fetch;
use bevy::ecs::query::WorldQuery;

pub trait OtherWorldQuery<const N: usize>{
    type Fetch: WorldQuery;
}

impl<const N: usize> OtherWorldQuery<N> for (){
    type Fetch = OtherFetch<(), N>;
}

pub struct OtherFetch<T, const N: usize>{
    p: std::marker::PhantomData<T>
}

unsafe impl<T, const N: usize> Send for OtherFetch<T, N>{}
unsafe impl<T, const N: usize> Sync for OtherFetch<T, N>{}

impl<const N: usize> WorldQuery for OtherFetch<(), N>{
    type Fetch = ();
    type State = ();
}

impl<T: Component, const N: usize> WorldQuery for OtherFetch<&T, N>{
    type Fetch = OtherReadFetch<T, N>;
    type State = OtherReadState<T, N>;
}

pub struct OtherReadFetch<T, const N: usize>{
    fetch: ReadFetch<T>
}

pub struct OtherReadState<T, const N: usize>{
    state: ReadState<T>
}

impl<'w, T: Component, const N: usize> Fetch<'w> for OtherReadFetch<T, N>{
    type Item = &'w T;
    type State = OtherReadState<T, N>;

    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_changetick: u32,
        change_tick: u32
    ) -> Self{   
        let fetch = ReadFetch::init(
            world.get_resource::<OtherWorld<N>>().expect(format!("You don't have an Otherworld<{}> in your resources!", N)),
            &state.state,
            last_changetick,
            change_tick,
        );
        Self{
            fetch
        }
    }
}

unsafe impl<T, const N: usize> ReadOnlyFetch for OtherReadFetch<T, N>{}