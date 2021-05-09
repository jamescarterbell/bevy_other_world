use crate::other::Otherable;
use bevy::ecs::component::Component;
use bevy::ecs::world::World;
use core::ops::DerefMut;
use bevy::ecs::world::Mut;
use bevy::ecs::archetype::ArchetypeId;
use bevy::ecs::archetype::Archetypes;
use bevy::ecs::storage::Tables;
use bevy::ecs::query::FilterFetch;
use bevy::ecs::query::WorldQuery;
use bevy::ecs::query::Fetch;
use bevy::ecs::storage::TableId;

use crate::other_query_state::OtherQueryState;

pub struct OtherQueryIter<'w, 's, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W>, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s OtherQueryState<W, Q, F>,
    world: &'w Mut<'w, W>,
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: Q::Fetch,
    filter: F::Fetch,
    is_dense: bool,
    current_len: usize,
    current_index: usize,
}

impl<'w, 's, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W>, F: WorldQuery> OtherQueryIter<'w, 's, W, Q, F>
where
    F::Fetch: FilterFetch,
{
    pub(crate) unsafe fn new(
        world: &'w Mut<'w, W>,
        query_state: &'s OtherQueryState<W, Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let fetch = <Q::Fetch as Fetch>::init(
            &world,
            &query_state.fetch_state,
            last_change_tick,
            change_tick,
        );
        let filter = <F::Fetch as Fetch>::init(
            &world,
            &query_state.filter_state,
            last_change_tick,
            change_tick,
        );
        OtherQueryIter {
            is_dense: fetch.is_dense() && filter.is_dense(),
            world,
            query_state,
            fetch,
            filter,
            tables: &world.storages().tables,
            archetypes: &world.archetypes(),
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            current_len: 0,
            current_index: 0,
        }
    }
}

impl<'w, 's, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W>, F: WorldQuery> Iterator for OtherQueryIter<'w, 's, W, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = <Q::Fetch as Fetch<'w>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.is_dense {
                loop {
                    if self.current_index == self.current_len {
                        let table_id = self.table_id_iter.next()?;
                        let table = &self.tables[*table_id];
                        self.fetch.set_table(&self.query_state.fetch_state, table);
                        self.filter.set_table(&self.query_state.filter_state, table);
                        self.current_len = table.len();
                        self.current_index = 0;
                        continue;
                    }

                    if !self.filter.table_filter_fetch(self.current_index) {
                        self.current_index += 1;
                        continue;
                    }

                    let item = self.fetch.table_fetch(self.current_index);

                    self.current_index += 1;
                    return Some(item);
                }
            } else {
                loop {
                    if self.current_index == self.current_len {
                        let archetype_id = self.archetype_id_iter.next()?;
                        let archetype = &self.archetypes[*archetype_id];
                        self.fetch.set_archetype(
                            &self.query_state.fetch_state,
                            archetype,
                            self.tables,
                        );
                        self.filter.set_archetype(
                            &self.query_state.filter_state,
                            archetype,
                            self.tables,
                        );
                        self.current_len = archetype.len();
                        self.current_index = 0;
                        continue;
                    }

                    if !self.filter.archetype_filter_fetch(self.current_index) {
                        self.current_index += 1;
                        continue;
                    }

                    let item = self.fetch.archetype_fetch(self.current_index);
                    self.current_index += 1;
                    return Some(item);
                }
            }
        }
    }

    // NOTE: For unfiltered Queries this should actually return a exact size hint,
    // to fulfil the ExactSizeIterator invariant, but this isn't practical without specialization.
    // For more information see Issue #1686.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let max_size = self
            .query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes()[ArchetypeId::new(index)].len())
            .sum();

        (0, Some(max_size))
    }
}

// NOTE: We can cheaply implement this for unfiltered Queries because we have:
// (1) pre-computed archetype matches
// (2) each archetype pre-computes length
// (3) there are no per-entity filters
// TODO: add an ArchetypeOnlyFilter that enables us to implement this for filters like With<T>
impl<'w, 's, W: DerefMut<Target = World> + Component, Q: WorldQuery + Otherable<W>> ExactSizeIterator for OtherQueryIter<'w, 's, W, Q, ()> {
    fn len(&self) -> usize {
        self.query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes()[ArchetypeId::new(index)].len())
            .sum()
    }
}