use crate::other::Otherable;
use bevy::ecs::world::Mut;
use bevy::ecs::system::QueryComponentError;
use core::any::TypeId;
use bevy::ecs::system::QuerySingleError;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::QueryEntityError;
use bevy::tasks::TaskPool;
use bevy::ecs::query::ReadOnlyFetch;
use bevy::ecs::query::QueryIter;
use bevy::ecs::system::SystemState;
use bevy::ecs::system::Query;
use bevy::ecs::system::SystemParamState;
use bevy::ecs::system::SystemParamFetch;
use bevy::ecs::system::SystemParam;
use bevy::ecs::query::FilterFetch;
use bevy::ecs::component::Component;
use bevy::ecs::world::World;
use core::ops::DerefMut;
use bevy::ecs::query::WorldQuery;
use bevy::ecs::query::Fetch;
use std::clone::Clone;

use crate::other_query_state::OtherQueryState;
use crate::other_query_iter::OtherQueryIter;

pub struct OtherQuery<'w, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W> + 'static, F: WorldQuery + 'static = ()>
where
    F::Fetch: FilterFetch,
{
    pub(crate) world: Mut<'w, W>,
    pub(crate) state: &'w OtherQueryState<W, Q, F>,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'w, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W> + 'static, F: WorldQuery + 'static> SystemParam for OtherQuery<'w, W, Q, F>
where
    F::Fetch: FilterFetch,{

    type Fetch = OtherQueryState<W, Q, F>;
}


impl<'w, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W>, F: WorldQuery> OtherQuery<'w, W, Q, F>
where
    F::Fetch: FilterFetch,
{
    /// # Safety
    /// This will create a Query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the Queries have unique mutable access.
    #[inline]
    pub(crate) unsafe fn new(
        world: Mut<'w, W>,
        state: &'w OtherQueryState<W, Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            world,
            state,
            last_change_tick,
            change_tick,
        }
    }

    /// Iterates over the query results. This can only be called for read-only queries
    #[inline]
    pub fn iter(&self) -> OtherQueryIter<'_, '_, W, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(&self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Iterates over the query results
    #[inline]
    pub fn iter_mut(&mut self) -> OtherQueryIter<'_, '_, W, Q, F> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(&self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Iterates over the query results
    ///
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn iter_unsafe(&self) -> OtherQueryIter<'_, '_, W, Q, F> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .iter_unchecked_manual(&self.world, self.last_change_tick, self.change_tick)
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal iterator. This can only be called for read-only queries
    #[inline]
    pub fn for_each(&self, f: impl FnMut(<Q::Fetch as Fetch<'_>>::Item))
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                &self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal iterator.
    #[inline]
    pub fn for_each_mut(&self, f: impl FnMut(<Q::Fetch as Fetch<'_>>::Item)) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                &self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    #[inline]
    pub fn par_for_each(
        &self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'_>>::Item) + Send + Sync + Clone,
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                &self.world,
                task_pool,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    #[inline]
    pub fn par_for_each_mut(
        &mut self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'_>>::Item) + Send + Sync + Clone,
    ) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                &self.world,
                task_pool,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Gets the query result for the given `entity`
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                &self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Gets the query result for the given `entity`
    #[inline]
    pub fn get_mut(
        &mut self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                &self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Gets the query result for the given `entity`
    ///
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .get_unchecked_manual(&self.world, entity, self.last_change_tick, self.change_tick)
    }

    /// Gets a reference to the entity's component of the given type. This will fail if the entity
    /// does not have the given component type or if the given component type does not match
    /// this query.
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryComponentError> {
        let world = &self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_read(archetype_component)
        {
            entity_ref
                .get::<T>()
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingReadAccess)
        }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the
    /// entity does not have the given component type or if the given component type does not
    /// match this query.
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFE: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the
    /// entity does not have the given component type or the component does not match the query.
    ///
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_component_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        let world = &self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_write(archetype_component)
        {
            entity_ref
                .get_unchecked_mut::<T>(self.last_change_tick, self.change_tick)
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingWriteAccess)
        }
    }

    pub fn single(&self) -> Result<<Q::Fetch as Fetch<'_>>::Item, QuerySingleError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        let mut query = self.iter();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }

    /// See [`Query::single`]
    pub fn single_mut(&mut self) -> Result<<Q::Fetch as Fetch<'_>>::Item, QuerySingleError> {
        let mut query = self.iter_mut();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }
}