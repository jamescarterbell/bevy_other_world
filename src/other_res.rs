use core::ops::Deref;
use crate::other::Other;
use bevy::ecs::system::SystemParamFetch;
use bevy::ecs::system::SystemParamState;
use bevy::ecs::system::SystemState;
use core::marker::PhantomData;
use bevy::ecs::component::ComponentId;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::World;
use core::ops::DerefMut;
use bevy::ecs::component::Component;
use bevy::ecs::component::ComponentTicks;

pub struct OtherRes<'w, W: DerefMut<Target = World> + Component, T: Component> {
    value: &'w T,
    ticks: &'w ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
    w: PhantomData<W>,
}

impl<'w, W: DerefMut<Target = World> + Component, T: Component> OtherRes<'w, W, T> {
    /// Returns true if (and only if) this resource been added since the last execution of this
    /// system.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if (and only if) this resource been changed since the last execution of this
    /// system.
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

impl<'a, W: DerefMut<Target = World> + Component, T: Component> SystemParam for OtherRes<'a, W, T> {
    type Fetch = OtherResState<W, T>;
}

pub struct OtherResState<W: DerefMut<Target = World> + Component, T> {
    component_id: ComponentId,
    marker: PhantomData<(T, W)>,
}

unsafe impl<W: DerefMut<Target = World> + Component, T: Component> SystemParamState for OtherResState<W, T> {
    type Config = ();

    fn init(world: &mut World, system_state: &mut SystemState, _config: Self::Config) -> Self {
        let outer_component_id = world.initialize_resource::<Other<W, T>>();
        let world_id = world.components_mut().get_or_insert_id::<W>();
        let mut world = unsafe{ world.get_resource_unchecked_mut::<W>().expect("Couldn't find world!") };
        let component_id = world.initialize_resource::<T>();
        let combined_access = system_state.component_access_set.combined_access_mut();
        if combined_access.has_write(component_id) {
            panic!(
                "OtherRes<{}, {}> in system {} conflicts with a previous ResMut<{0}, {1}> access. Allowing this would break Rust's mutability rules. Consider removing the duplicate access.",
                std::any::type_name::<W>(), std::any::type_name::<T>(), system_state.name);
        }
        combined_access.add_read(outer_component_id);
        combined_access.add_write(world_id);

        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'a, W: DerefMut<Target = World> + Component, T: Component> SystemParamFetch<'a> for OtherResState<W, T> {
    type Item = OtherRes<'a, W, T>;

    #[inline]
    unsafe fn get_param(
        state: &'a mut Self,
        system_state: &'a SystemState,
        world: &'a World,
        change_tick: u32,
    ) -> Self::Item {
        let mut world = world.get_resource_unchecked_mut::<W>().expect("Couldn't find world!");
        let column = world
            .get_populated_resource_column(state.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_state.name,
                    std::any::type_name::<T>()
                )
            });
        OtherRes {
            value: &*column.get_ptr().as_ptr().cast::<T>(),
            ticks: &*column.get_ticks_mut_ptr(),
            last_change_tick: system_state.last_change_tick,
            change_tick,
            w: PhantomData
        }
    }
}

impl<'w, W: DerefMut<Target = World> + Component, T: Component> Deref for OtherRes<'w, W, T> {
    type Target = T;

    
    fn deref(&self) -> &<Self as std::ops::Deref>::Target { 
        &self.value
    }
}