use crate::{
    DefComponent, DefIndex, DefRef,
    debug::{debug_checked_unwrap_option, debug_checked_unwrap_result},
};
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    archetype::Archetype,
    component::{ComponentId, Components, Tick},
    entity::Entity,
    query::{FilteredAccess, QueryData, ReadOnlyQueryData, WorldQuery},
    storage::{Table, TableRow},
    world::{
        World,
        unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
    },
};
use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
};

pub struct DefEntityRef<'w, T: DefComponent> {
    entity: UnsafeEntityCell<'w>,
    asset: &'w Assets<<T as DefComponent>::Asset>,
    index: &'w DefIndex<T>,
}

impl<'w, T: DefComponent> DefEntityRef<'w, T> {
    pub fn find<Q>(&self, name: &Q) -> Option<(AssetId<T::Asset>, ComponentId)>
    where
        Q: Hash + Eq + ?Sized,
        Cow<'static, str>: Borrow<Q>,
    {
        self.index.find_by_name(name)
    }

    pub fn component_id(&self, id: impl Into<AssetId<T::Asset>>) -> Option<ComponentId> {
        self.index.asset_to_id.get(&id.into()).copied()
    }

    pub fn asset_id(&self, id: ComponentId) -> Option<AssetId<T::Asset>> {
        self.index.id_to_asset.get(&id).copied()
    }

    pub fn find_ref<Q>(&self, name: &Q) -> Option<DefRef<'w, '_, T>>
    where
        Q: Hash + Eq + ?Sized,
        Cow<'static, str>: Borrow<Q>,
    {
        let (asset_id, component_id) = self.find(name)?;
        let asset = self.asset.get(asset_id)?;
        let value = unsafe { self.value_ref(component_id)? };
        Some(DefRef { value, asset })
    }

    pub fn get_ref(&self, id: impl Into<AssetId<T::Asset>>) -> Option<DefRef<'w, '_, T>> {
        let asset_id = id.into();
        let component_id = self.index.asset_to_id.get(&asset_id).copied()?;
        let asset = self.asset.get(asset_id)?;
        let value = unsafe { self.value_ref(component_id)? };
        Some(DefRef { value, asset })
    }

    pub fn asset(&self, id: impl Into<AssetId<T::Asset>>) -> Option<&'_ T::Asset> {
        self.asset.get(id)
    }

    /// # Safety
    /// no
    pub unsafe fn value_ref(&self, id: ComponentId) -> Option<&'w T> {
        unsafe {
            let ptr = self.entity.get_by_id(id)?;
            Some(ptr.deref())
        }
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<'a, T: DefComponent> QueryData for DefEntityRef<'a, T> {
    const IS_READ_ONLY: bool = true;

    type ReadOnly = Self;
    type Item<'w> = DefEntityRef<'w, T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        world: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let entity = unsafe { debug_checked_unwrap_result(world.get_entity(entity)) };

        let index = unsafe { debug_checked_unwrap_option(world.get_resource::<DefIndex<T>>()) };
        let asset =
            unsafe { debug_checked_unwrap_option(world.get_resource::<Assets<T::Asset>>()) };

        // SAFETY: mutable access to every component has been registered.
        unsafe {
            DefEntityRef {
                entity,
                index,
                asset,
            }
        }
    }
}

/// SAFETY: Access is read-only.
unsafe impl<T: DefComponent> ReadOnlyQueryData for DefEntityRef<'_, T> {}

/// SAFETY: The accesses of `Self::ReadOnly` are a subset of the accesses of `Self`
unsafe impl<'a, T: DefComponent> WorldQuery for DefEntityRef<'a, T> {
    type Fetch<'w> = UnsafeWorldCell<'w>;

    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    const IS_DENSE: bool = false;

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        // let access = Access::default();
        // // access.read_all_components();
        // (world, access)
        world
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        _: &'w Archetype,
        _table: &Table,
    ) {
        // fetch.1.clone_from(state.access());
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, _: &'w Table) {
        // fetch.1.clone_from(state.access());
    }

    #[inline]
    fn set_access<'w>(state: &mut Self::State, access: &FilteredAccess<ComponentId>) {
        // state.clone_from(access);
        // state.access_mut().clear_writes();
    }

    fn update_component_access(
        state: &Self::State,
        filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        // assert!(
        //     filtered_access.access().is_compatible(state.access()),
        //     "DefRef conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        // );

        // filtered_access.access_mut().extend(state.access());
    }

    fn init_state(_world: &mut World) -> Self::State {
        // FilteredAccess::default()
    }

    fn get_state(_components: &Components) -> Option<Self::State> {
        // Some(FilteredAccess::default())
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}
