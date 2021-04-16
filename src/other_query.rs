use bevy::ecs::query::WorldQuery;
use bevy::ecs::query::QueryState;
use bevy::ecs::query::FilterFetch;
use bevy::ecs::system::Query;
use bevy::ecs::system::SystemParam;
use crate::other_world_query::{OtherFetch, OtherWorldQuery};

/// Provides scoped access to an OtherWorld according to a given WorldQuery and query filter
pub struct OtherQuery<'w, Q, F, const N: usize>
where
    F: OtherWorldQuery<N>,
    Q: OtherWorldQuery<N>,
    <<F as OtherWorldQuery<N>>::Fetch as WorldQuery>::Fetch : FilterFetch,
{
    pub(crate) state: Query<'w, <Q as OtherWorldQuery<N>>::Fetch, <F as OtherWorldQuery<N>>::Fetch>,
}

impl<'a, Q: OtherWorldQuery<N> + 'static, F: OtherWorldQuery<N> + 'static, const N: usize> SystemParam for OtherQuery<'a, Q, F, N>
where
    <<F as OtherWorldQuery<N>>::Fetch as WorldQuery>::Fetch : FilterFetch,
{
    type Fetch = QueryState<<Q as OtherWorldQuery<N>>::Fetch, <F as OtherWorldQuery<N>>::Fetch>;
}