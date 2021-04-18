use core::any::TypeId;
use bevy::ecs::storage::Tables;
use bevy::ecs::storage::ComponentSparseSet;
use bevy::ecs::entity::Entity;
use core::ptr::NonNull;
use core::ptr;
use bevy::ecs::storage::Table;
use bevy::ecs::archetype::ArchetypeComponentId;
use bevy::ecs::query::Access;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::query::FilteredAccess;
use std::marker::PhantomData;
use bevy::ecs::component::StorageType;
use bevy::ecs::component::ComponentId;
use crate::other_world::OtherWorld;
use bevy::ecs::world::World;
use bevy::ecs::query::ReadOnlyFetch;
use bevy::ecs::query::ReadState;
use bevy::ecs::query::ReadFetch;
use bevy::ecs::component::Component;
use bevy::ecs::query::FetchState;
use bevy::ecs::query::Fetch;
use bevy::ecs::query::WorldQuery;

pub struct Other<T, const N: usize>{
    data: PhantomData<T>,
}

impl<const N: usize> Otherable<N> for (){
    type Fetch = ();
    type State = ();
}

pub trait Otherable<const N: usize>{
    type Fetch: for<'a> Fetch<'a, State = Self::State>;
    type State: FetchState;
}

impl<T: Component, const N: usize> Otherable<N> for &T{
    type Fetch = OtherReadFetch<T, N>;
    type State = OtherReadState<T, N>;
}

impl<T1: Otherable<N>, T2: Otherable<N>, const N: usize> Otherable<N> for (T1, T2){
    type Fetch = (T1::Fetch, T2::Fetch);
    type State = (T1::State, T2::State);
}

impl<T: Otherable<N>, const N: usize> WorldQuery for Other<T, N>{
    type Fetch = <T as Otherable<N>>::Fetch;
    type State = <T as Otherable<N>>::State;
}

pub struct OtherReadFetch<T, const N: usize>{
    storage_type: StorageType,
    table_components: NonNull<T>,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
}

impl<'w, T: Component, const N: usize> Fetch<'w> for OtherReadFetch<T, N>{
    type Item = &'w T;
    type State = OtherReadState<T, N>;

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
        };
        if state.storage_type == StorageType::SparseSet {
            value.sparse_set = world
                .get_resource::<OtherWorld<N>>()
                .expect(&format!("You don't have an Otherworld<{}> in your resources!", N))
                .storages()
                .sparse_sets
                .get(state.other_component_id)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.other_component_id)
                    .unwrap();
                self.table_components = column.get_ptr().cast::<T>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.table_components = table
            .get_column(state.other_component_id)
            .unwrap()
            .get_ptr()
            .cast::<T>();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                &*self.table_components.as_ptr().add(table_row)
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                &*(*self.sparse_set).get(entity).unwrap().cast::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        &*self.table_components.as_ptr().add(table_row)
    }
}

unsafe impl<T: Component, const N: usize> ReadOnlyFetch for OtherReadFetch<T, N>{}

pub struct OtherReadState<T, const N: usize>{
    component_id: ComponentId,
    storage_type: StorageType,
    other_component_id: ComponentId,
    other_storage_type: StorageType,
    marker: PhantomData<T>,
}

unsafe impl<T: Component, const N: usize> FetchState for OtherReadState<T, N>{
    fn init(world: &mut World) -> Self {
        world
            .get_resource_mut::<OtherWorld<N>>()
            .expect(&format!("You don't have an Otherworld<{}> in your resources!", N))
            .components_mut()
            .get_or_insert_info::<T>();
            
        world
            .components_mut()
            .get_or_insert_info::<Other<T, N>>();

        let component_info = {
                let id = world.components().get_id(TypeId::of::<Other<T, N>>()).unwrap();
                unsafe{world.components().get_info_unchecked(id)}
            };

        let other_component_info = {
                let other_world = world
                    .get_resource::<OtherWorld<N>>()
                    .unwrap();
                let id = other_world.components().get_id(TypeId::of::<Other<T, N>>()).unwrap();
                unsafe{other_world.components().get_info_unchecked(id)}
            };

        Self {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
            other_component_id: other_component_info.id(),
            other_storage_type: other_component_info.storage_type(),
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        access.add_read(self.component_id)
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(self.component_id)
        {
            access.add_read(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.component_id)
    }
}
