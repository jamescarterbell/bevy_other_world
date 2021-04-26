use bevy::ecs::query::FilteredAccessSet;
use bevy::ecs::system::SystemParamState;
use bevy::ecs::system::SystemState;
use bevy::ecs::system::SystemParamFetch;
use bevy::ecs::component::Component;
use core::ops::DerefMut;
use bevy::ecs::world::Mut;
use bevy::tasks::TaskPool;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::query::ReadOnlyFetch;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::ecs::archetype::ArchetypeId;
use bevy::ecs::storage::TableId;
use bevy::ecs::component::ComponentId;
use bevy::ecs::query::FilteredAccess;
use bevy::ecs::query::Access;
use bevy::ecs::archetype::ArchetypeComponentId;
use fixedbitset::FixedBitSet;
use bevy::ecs::archetype::ArchetypeGeneration;
use bevy::ecs::world::WorldId;
use bevy::ecs::query::FilterFetch;
use bevy::ecs::query::WorldQuery;
use bevy::ecs::query::Fetch;
use bevy::ecs::query::FetchState;

use crate::other_query_iter::OtherQueryIter;
use crate::other_query::OtherQuery;

pub struct OtherQueryState<W, Q: WorldQuery, F: WorldQuery = ()>
where
    F::Fetch: FilterFetch,
{
    world_id: WorldId,
    pub(crate) archetype_generation: ArchetypeGeneration,
    pub(crate) matched_tables: FixedBitSet,
    pub(crate) matched_archetypes: FixedBitSet,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a TableId bitset and a vec because iterating the vec is faster
    pub(crate) matched_table_ids: Vec<TableId>,
    // NOTE: we maintain both a ArchetypeId bitset and a vec because iterating the vec is faster
    pub(crate) matched_archetype_ids: Vec<ArchetypeId>,
    pub(crate) fetch_state: Q::State,
    pub(crate) filter_state: F::State,
    w: std::marker::PhantomData<W>
}

impl<'w, W: DerefMut<Target = World> + Component, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamFetch<'w> for OtherQueryState<W, Q, F>
where
    F::Fetch: FilterFetch,{

    type Item = OtherQuery<'w, W, Q, F>;

    #[inline]
    unsafe fn get_param(
        state: &'w mut Self,
        system_state: &'w SystemState,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let last_change_tick = world.last_change_tick();
        let change_tick = world.read_change_tick();
        let world = world.get_resource_unchecked_mut::<W>().expect("Couldn't find world!");
        OtherQuery::new(world, state, last_change_tick, change_tick)
    }
}

unsafe impl<'w, W: DerefMut<Target = World> + Component, Q: WorldQuery + 'static, F: WorldQuery + 'static> SystemParamState for OtherQueryState<W, Q, F>
where
    F::Fetch: FilterFetch,{

    type Config = ();
    

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        let state = OtherQueryState::new(world);
        assert_component_access_compatibility(
            &system_state.name,
            std::any::type_name::<Q>(),
            std::any::type_name::<F>(),
            &system_state.component_access_set,
            &state.component_access,
            world,
        );
        system_state
            .component_access_set
            .add(state.component_access.clone());
        system_state
            .archetype_component_access
            .extend(&state.archetype_component_access);
        state
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_state: &mut SystemState) {
        self.new_archetype(archetype);
        system_state
            .archetype_component_access
            .extend(&self.archetype_component_access);
    }

    fn default_config() {}

}

fn assert_component_access_compatibility(
    system_name: &str,
    query_type: &'static str,
    filter_type: &'static str,
    system_access: &FilteredAccessSet<ComponentId>,
    current: &FilteredAccess<ComponentId>,
    world: &World,
) {
    let mut conflicts = system_access.get_conflicts(current);
    if conflicts.is_empty() {
        return;
    }
    let conflicting_components = conflicts
        .drain(..)
        .map(|component_id| world.components().get_info(component_id).unwrap().name())
        .collect::<Vec<&str>>();
    let accesses = conflicting_components.join(", ");
    panic!("Query<{}, {}> in system {} accesses component(s) {} in a way that conflicts with a previous system parameter. Allowing this would break Rust's mutability rules. Consider merging conflicting Queries into a QuerySet.",
                query_type, filter_type, system_name, accesses);
}

impl<W: DerefMut<Target = World> + Component, Q: WorldQuery, F: WorldQuery> OtherQueryState<W, Q, F>
where
    F::Fetch: FilterFetch,
{
    pub fn new(world: &mut World) -> Self {
        let fetch_state = <Q::State as FetchState>::init(world);
        let filter_state = <F::State as FetchState>::init(world);
        let mut component_access = Default::default();
        fetch_state.update_component_access(&mut component_access);
        filter_state.update_component_access(&mut component_access);
        let mut state = Self {
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::new(usize::MAX),
            matched_table_ids: Vec::new(),
            matched_archetype_ids: Vec::new(),
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            archetype_component_access: Default::default(),
            w: std::marker::PhantomData,
        };
        state.validate_world_and_update_archetypes(&world);
        state
    }

    pub fn validate_world_and_update_archetypes(&mut self, world: &World) {
        if world.id() != self.world_id {
            panic!("Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>());
        }
        let archetypes = world.archetypes();
        let old_generation = self.archetype_generation;
        let archetype_index_range = if old_generation == archetypes.generation() {
            0..0
        } else {
            self.archetype_generation = archetypes.generation();
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
        if self.fetch_state.matches_archetype(archetype)
            && self.filter_state.matches_archetype(archetype)
        {
            self.fetch_state
                .update_archetype_component_access(archetype, &mut self.archetype_component_access);
            self.filter_state
                .update_archetype_component_access(archetype, &mut self.archetype_component_access);
            let archetype_index = archetype.id().index();
            if !self.matched_archetypes.contains(archetype_index) {
                self.matched_archetypes.grow(archetype_index + 1);
                self.matched_archetypes.set(archetype_index, true);
                self.matched_archetype_ids.push(archetype.id());
            }
            let table_index = archetype.table_id().index();
            if !self.matched_tables.contains(table_index) {
                self.matched_tables.grow(table_index + 1);
                self.matched_tables.set(table_index, true);
                self.matched_table_ids.push(archetype.table_id());
            }
        }
    }

    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        self.validate_world_and_update_archetypes(&world);
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
        world: &'w Mut<'w, W>,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        let location = world
            .entities()
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity)?;
        if !self
            .matched_archetypes
            .contains(location.archetype_id.index())
        {
            return Err(QueryEntityError::QueryDoesNotMatch);
        }
        let archetype = &world.archetypes()[location.archetype_id];
        let mut fetch =
            <Q::Fetch as Fetch>::init(&world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(&world, &self.filter_state, last_change_tick, change_tick);

        fetch.set_archetype(&self.fetch_state, archetype, &world.storages().tables);
        filter.set_archetype(&self.filter_state, archetype, &world.storages().tables);
        if filter.archetype_filter_fetch(location.index) {
            Ok(fetch.archetype_fetch(location.index))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch)
        }
    }

    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w Mut<'w, W>) -> OtherQueryIter<'w, 's, W, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe { self.iter_unchecked(world) }
    }

    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w Mut<'w, W>) -> OtherQueryIter<'w, 's, W, Q, F> {
        // SAFE: query has unique world access
        unsafe { self.iter_unchecked(world) }
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_unchecked<'w, 's>(
        &'s mut self,
        world: &'w Mut<'w, W>,
    ) -> OtherQueryIter<'w, 's, W, Q, F> {
        self.validate_world_and_update_archetypes(&world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    #[inline]
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w Mut<'w, W>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> OtherQueryIter<'w, 's, W, Q, F> {
        OtherQueryIter::new(world, self, last_change_tick, change_tick)
    }

    #[inline]
    pub fn for_each<'w>(
        &mut self,
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
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
        world: &'w Mut<'w, W>,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        self.validate_world_and_update_archetypes(&world);
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
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    pub(crate) unsafe fn for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w Mut<'w, W>,
        mut func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
        last_change_tick: u32,
        change_tick: u32,
    ) {
        let mut fetch =
            <Q::Fetch as Fetch>::init(&world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(&world, &self.filter_state, last_change_tick, change_tick);
        if fetch.is_dense() && filter.is_dense() {
            let tables = &world.storages().tables;
            for table_id in self.matched_table_ids.iter() {
                let table = &tables[*table_id];
                fetch.set_table(&self.fetch_state, table);
                filter.set_table(&self.filter_state, table);

                for table_index in 0..table.len() {
                    if !filter.table_filter_fetch(table_index) {
                        continue;
                    }
                    let item = fetch.table_fetch(table_index);
                    func(item);
                }
            }
        } else {
            let archetypes = &world.archetypes();
            let tables = &world.storages().tables;
            for archetype_id in self.matched_archetype_ids.iter() {
                let archetype = &archetypes[*archetype_id];
                fetch.set_archetype(&self.fetch_state, archetype, tables);
                filter.set_archetype(&self.filter_state, archetype, tables);

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
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsafe.
    pub unsafe fn par_for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w Mut<'w, W>,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        task_pool.scope(|scope| {
            let fetch =
                <Q::Fetch as Fetch>::init(&world, &self.fetch_state, last_change_tick, change_tick);
            let filter =
                <F::Fetch as Fetch>::init(&world, &self.filter_state, last_change_tick, change_tick);

            if fetch.is_dense() && filter.is_dense() {
                let tables = &world.storages().tables;
                for table_id in self.matched_table_ids.iter() {
                    let table = &tables[*table_id];
                    let mut offset = 0;
                    while offset < table.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                &world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = <F::Fetch as Fetch>::init(
                                &world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let table = &tables[*table_id];
                            fetch.set_table(&self.fetch_state, table);
                            filter.set_table(&self.filter_state, table);
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
                let archetypes = &world.archetypes();
                for archetype_id in self.matched_archetype_ids.iter() {
                    let mut offset = 0;
                    let archetype = &archetypes[*archetype_id];
                    while offset < archetype.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                &world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = <F::Fetch as Fetch>::init(
                                &world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let archetype = &world.archetypes()[*archetype_id];
                            fetch.set_archetype(&self.fetch_state, archetype, tables);
                            filter.set_archetype(&self.filter_state, archetype, tables);

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