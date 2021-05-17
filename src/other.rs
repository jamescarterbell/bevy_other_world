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

macro_rules! impl_tuple_fetch{
    ($($name: ident),*) => {
        impl<W: DerefMut<Target = World> + Component, $($name: Component + Otherable<W>,)*> Otherable<W> for ($($name,)*){
            type OtherState = ($($name::OtherState,)*);
        }
    }
}



impl_tuple_fetch!(T1);
impl_tuple_fetch!(T1, T2);
impl_tuple_fetch!(T1, T2, T3);
impl_tuple_fetch!(T1, T2, T3, T4);
impl_tuple_fetch!(T1, T2, T3, T4, T5);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_tuple_fetch!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);