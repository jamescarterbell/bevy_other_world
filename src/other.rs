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
    type State: FetchState;
}

impl<W: DerefMut<Target = World> + Component, T: Component> Otherable<W> for Other<W, &T>{
    type State = ReadState<Other<W, T>>;
}

impl<W: DerefMut<Target = World> + Component, T: Component> Otherable<W> for Other<W, &mut T>{
    type State = WriteState<Other<W, T>>;
}

impl<W: DerefMut<Target = World> + Component> Otherable<W> for Other<W, ()>{
    type State = ();
}