use bevy::ecs::archetype::Archetype;
use fixedbitset::FixedBitSet;
use crate::other_world::OtherWorld;
use bevy::ecs::world::World;
use bevy::ecs::archetype::ArchetypeId;
use bevy::ecs::component::ComponentId;
use bevy::ecs::archetype::ArchetypeComponentId;
use bevy::ecs::storage::TableId;
use bevy::ecs::query::FilteredAccess;
use bevy::ecs::query::Access;
use bevy::ecs::query::FetchState;
use bevy::ecs::query::Fetch;
use bevy::ecs::query::ReadOnlyFetch;
use bevy::ecs::archetype::ArchetypeGeneration;
use bevy::ecs::world::WorldId;
use bevy::ecs::query::FilterFetch;
use bevy::ecs::query::WorldQuery;


pub struct OtherQueryState<Q: WorldQuery, F: WorldQuery, const N: usize>
where
    F::Fetch: FilterFetch,
{
    other_world_id: WorldId,
    pub(crate) other_archetype_generation: ArchetypeGeneration,
    pub(crate) other_matched_tables: FixedBitSet,
    pub(crate) other_matched_archetypes: FixedBitSet,
    pub(crate) other_archetype_other_component_access: Access<ArchetypeComponentId>,
    pub(crate) archetype_other_component_access: Access<ArchetypeComponentId>,
    pub(crate) other_component_access: FilteredAccess<ComponentId>,
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a TableId bitset and a vec because iterating the vec is faster
    pub(crate) other_matched_table_ids: Vec<TableId>,
    // NOTE: we maintain both a ArchetypeId bitset and a vec because iterating the vec is faster
    pub(crate) other_matched_archetype_ids: Vec<ArchetypeId>,
    pub(crate) other_fetch_state: Q::State,
    pub(crate) other_filter_state: F::State,
}

impl<Q: WorldQuery, F: WorldQuery, const N: usize> OtherQueryState<Q, F, N>
where
    F::Fetch: FilterFetch,
{
    pub fn new(world: &mut World) -> Self {
        let other_world = world.get_resource_mut::<OtherWorld<N>>().unwrap();
        let other_fetch_state = <Q::State as FetchState>::init(&mut other_world);
        let other_filter_state = <F::State as FetchState>::init(&mut other_world);
        let mut other_component_access = Default::default();
        other_fetch_state.update_component_access(&mut other_component_access);
        other_filter_state.update_component_access(&mut other_component_access);
        let mut state = Self {
            other_world_id: other_world.id(),
            other_archetype_generation: ArchetypeGeneration::new(usize::MAX),
            other_matched_table_ids: Vec::new(),
            other_matched_archetype_ids: Vec::new(),
            other_fetch_state,
            other_filter_state,
            other_component_access,
            other_matched_tables: Default::default(),
            other_matched_archetypes: Default::default(),
            other_archetype_other_component_access: Default::default(),
        };
        state.validate_world_and_update_archetypes(&other_world);
        state
    }

    pub fn validate_world_and_update_archetypes(&mut self, world: &World) {
        if world.id() != self.other_world_id {
            panic!("Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>());
        }
        let archetypes = world.archetypes();
        let old_generation = self.other_archetype_generation;
        let archetype_index_range = if old_generation == archetypes.generation() {
            0..0
        } else {
            self.other_archetype_generation = archetypes.generation();
            if old_generation.value() == usize::MAX {
                0..archetypes.len()
            } else {
                old_generation.value()..archetypes.len()
            }
        };
        for archetype_index in archetype_index_range {
            self.new_archetype(&archetypes[ArchetypeId::new(archetype_index)]);
        }
    }

    pub fn new_archetype(&mut self, archetype: &Archetype) {
        if self.other_fetch_state.matches_archetype(archetype)
            && self.other_filter_state.matches_archetype(archetype)
        {
            self.other_fetch_state
                .update_archetype_component_access(archetype, &mut self.other_archetype_other_component_access);
            self.other_filter_state
                .update_archetype_component_access(archetype, &mut self.other_archetype_other_component_access);
            let archetype_index = archetype.id().index();
            if !self.other_matched_archetypes.contains(archetype_index) {
                self.other_matched_archetypes.grow(archetype_index + 1);
                self.other_matched_archetypes.set(archetype_index, true);
                self.other_matched_archetype_ids.push(archetype.id());
            }
            let table_index = archetype.table_id().index();
            if !self.other_matched_tables.contains(table_index) {
                self.other_matched_tables.grow(table_index + 1);
                self.other_matched_tables.set(table_index, true);
                self.other_matched_table_ids.push(archetype.table_id());
            }
        }
    }

    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe { self.get_unchecked(world, entity) }
    }

    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        // SAFE: query has unique world access
        unsafe { self.get_unchecked(world, entity) }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn get_unchecked<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        self.validate_world_and_update_archetypes(world);
        self.get_unchecked_manual(
            world,
            entity,
            world.last_change_tick(),
            world.read_change_tick(),
        )
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn get_unchecked_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        let location = world
            .entities
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity)?;
        if !self
            .other_matched_archetypes
            .contains(location.archetype_id.index())
        {
            return Err(QueryEntityError::QueryDoesNotMatch);
        }
        let archetype = &world.archetypes[location.archetype_id];
        let mut fetch =
            <Q::Fetch as Fetch>::init(world, &self.other_fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(world, &self.other_filter_state, last_change_tick, change_tick);

        fetch.set_archetype(&self.other_fetch_state, archetype, &world.storages().tables);
        filter.set_archetype(&self.other_filter_state, archetype, &world.storages().tables);
        if filter.archetype_filter_fetch(location.index) {
            Ok(fetch.archetype_fetch(location.index))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch)
        }
    }

    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> QueryIter<'w, 's, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe { self.iter_unchecked(world) }
    }

    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, Q, F> {
        // SAFE: query has unique world access
        unsafe { self.iter_unchecked(world) }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_unchecked<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q, F> {
        self.validate_world_and_update_archetypes(world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.other_world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    #[inline]
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryIter<'w, 's, Q, F> {
        QueryIter::new(world, self, last_change_tick, change_tick)
    }

    #[inline]
    pub fn for_each<'w>(
        &mut self,
        world: &'w World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe {
            self.for_each_unchecked(world, func);
        }
    }

    #[inline]
    pub fn for_each_mut<'w>(
        &mut self,
        world: &'w mut World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) {
        // SAFE: query has unique world access
        unsafe {
            self.for_each_unchecked(world, func);
        }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn for_each_unchecked<'w>(
        &mut self,
        world: &'w World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) {
        self.validate_world_and_update_archetypes(world);
        self.for_each_unchecked_manual(
            world,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    #[inline]
    pub fn par_for_each<'w>(
        &mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe {
            self.par_for_each_unchecked(world, task_pool, batch_size, func);
        }
    }

    #[inline]
    pub fn par_for_each_mut<'w>(
        &mut self,
        world: &'w mut World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        // SAFE: query has unique world access
        unsafe {
            self.par_for_each_unchecked(world, task_pool, batch_size, func);
        }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn par_for_each_unchecked<'w>(
        &mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        self.validate_world_and_update_archetypes(world);
        self.par_for_each_unchecked_manual(
            world,
            task_pool,
            batch_size,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.other_world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    pub(crate) unsafe fn for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        mut func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
        last_change_tick: u32,
        change_tick: u32,
    ) {
        let mut fetch =
            <Q::Fetch as Fetch>::init(world, &self.other_fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(world, &self.other_filter_state, last_change_tick, change_tick);
        if fetch.is_dense() && filter.is_dense() {
            let tables = &world.storages().tables;
            for table_id in self.other_matched_table_ids.iter() {
                let table = &tables[*table_id];
                fetch.set_table(&self.other_fetch_state, table);
                filter.set_table(&self.other_filter_state, table);

                for table_index in 0..table.len() {
                    if !filter.table_filter_fetch(table_index) {
                        continue;
                    }
                    let item = fetch.table_fetch(table_index);
                    func(item);
                }
            }
        } else {
            let archetypes = &world.archetypes;
            let tables = &world.storages().tables;
            for archetype_id in self.other_matched_archetype_ids.iter() {
                let archetype = &archetypes[*archetype_id];
                fetch.set_archetype(&self.other_fetch_state, archetype, tables);
                filter.set_archetype(&self.other_filter_state, archetype, tables);

                for archetype_index in 0..archetype.len() {
                    if !filter.archetype_filter_fetch(archetype_index) {
                        continue;
                    }
                    func(fetch.archetype_fetch(archetype_index));
                }
            }
        }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.other_world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    pub unsafe fn par_for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        task_pool.scope(|scope| {
            let fetch =
                <Q::Fetch as Fetch>::init(world, &self.other_fetch_state, last_change_tick, change_tick);
            let filter =
                <F::Fetch as Fetch>::init(world, &self.other_filter_state, last_change_tick, change_tick);

            if fetch.is_dense() && filter.is_dense() {
                let tables = &world.storages().tables;
                for table_id in self.other_matched_table_ids.iter() {
                    let table = &tables[*table_id];
                    let mut offset = 0;
                    while offset < table.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                world,
                                &self.other_fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = <F::Fetch as Fetch>::init(
                                world,
                                &self.other_filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let table = &tables[*table_id];
                            fetch.set_table(&self.other_fetch_state, table);
                            filter.set_table(&self.other_filter_state, table);
                            let len = batch_size.min(table.len() - offset);
                            for table_index in offset..offset + len {
                                if !filter.table_filter_fetch(table_index) {
                                    continue;
                                }
                                let item = fetch.table_fetch(table_index);
                                func(item);
                            }
                        });
                        offset += batch_size;
                    }
                }
            } else {
                let archetypes = &world.archetypes;
                for archetype_id in self.other_matched_archetype_ids.iter() {
                    let mut offset = 0;
                    let archetype = &archetypes[*archetype_id];
                    while offset < archetype.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                world,
                                &self.other_fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = <F::Fetch as Fetch>::init(
                                world,
                                &self.other_filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let archetype = &world.archetypes[*archetype_id];
                            fetch.set_archetype(&self.other_fetch_state, archetype, tables);
                            filter.set_archetype(&self.other_filter_state, archetype, tables);

                            for archetype_index in 0..archetype.len() {
                                if !filter.archetype_filter_fetch(archetype_index) {
                                    continue;
                                }
                                func(fetch.archetype_fetch(archetype_index));
                            }
                        });
                        offset += batch_size;
                    }
                }
            }
        });
    }
}