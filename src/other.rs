use bevy::ecs::query::WriteState;
use bevy::ecs::world::World;
use core::ops::DerefMut;
use bevy::ecs::query::ReadState;
use bevy::ecs::query::ReadFetch;
use bevy::ecs::component::Component;
use bevy::ecs::query::FetchState;
use bevy::ecs::query::Fetch;
use bevy::ecs::query::WorldQuery;
use core::marker::PhantomData;

pub struct Other<W: DerefMut<Target = World> + Component, T>{
    data: PhantomData<(T, W)>,
}

pub trait Otherable<W: DerefMut<Target = World> + Component>{
    type OtherState: FetchState;
}

impl<W: DerefMut<Target = World> + Component, T: Component> Otherable<W> for &T{
    type OtherState = ReadState<Other<W, T>>;
}

impl<W: DerefMut<Target = World> + Component, T: Component> Otherable<W> for &mut T{
    type OtherState = WriteState<Other<W, T>>;
}

impl<W: DerefMut<Target = World> + Component> Otherable<W> for (){
    type OtherState = ();
}


impl<W: DerefMut<Target = World> + Component, T1: Component + Otherable<W>, T2: Component + Otherable<W>> Otherable<W> for (T1, T2){
    type OtherState = (T1::OtherState, T2::OtherState);
}