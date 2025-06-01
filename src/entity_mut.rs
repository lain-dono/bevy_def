use crate::{
    DefComponent, DefEntityRef, DefIndex, DefMut, DefQueryState, DefRef,
    debug::debug_checked_unwrap_result,
};
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    archetype::Archetype,
    component::{ComponentId, Components, Tick},
    entity::Entity,
    query::{FilteredAccess, QueryData, WorldQuery},
    storage::{Table, TableRow},
    world::{
        Mut, World,
        unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
    },
};
use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
};

pub struct DefEntityMut<'w, T: DefComponent> {
    entity: UnsafeEntityCell<'w>,
    asset: &'w Assets<<T as DefComponent>::Asset>,
    index: &'w DefIndex<T>,
}

impl<'w, T: DefComponent> DefEntityMut<'w, T> {
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

    pub fn find_mut<Q>(&mut self, name: &Q) -> Option<DefMut<'w, '_, T>>
    where
        Q: Hash + Eq + ?Sized,
        Cow<'static, str>: Borrow<Q>,
    {
        let (asset_id, component_id) = self.find(name)?;
        let asset = self.asset.get(asset_id)?;
        let value = unsafe { self.value_mut(component_id)? };
        Some(DefMut { value, asset })
    }

    pub fn get_ref(&self, id: impl Into<AssetId<T::Asset>>) -> Option<DefRef<'w, '_, T>> {
        let asset_id = id.into();
        let component_id = self.index.asset_to_id.get(&asset_id).copied()?;
        let asset = self.asset.get(asset_id)?;
        let value = unsafe { self.value_ref(component_id)? };
        Some(DefRef { value, asset })
    }

    pub fn get_mut(&mut self, id: impl Into<AssetId<T::Asset>>) -> Option<DefMut<'w, '_, T>> {
        let asset_id = id.into();
        let component_id = self.index.asset_to_id.get(&asset_id).copied()?;
        let asset = self.asset.get(asset_id)?;
        let value = unsafe { self.value_mut(component_id)? };
        Some(DefMut { value, asset })
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

    /// # Safety
    /// no
    pub unsafe fn value_mut(&self, id: ComponentId) -> Option<Mut<'w, T>> {
        unsafe {
            let ptr = self.entity.get_mut_by_id(id).ok()?;
            Some(ptr.with_type())
        }
    }
}

/// SAFETY: The accesses of `Self::ReadOnly` are a subset of the accesses of `Self`
unsafe impl<'a, T: DefComponent> WorldQuery for DefEntityMut<'a, T> {
    type Fetch<'w> = (UnsafeWorldCell<'w>, &'w DefIndex<T>, &'w Assets<T::Asset>);
    type State = DefQueryState;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    const IS_DENSE: bool = false;

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        // let access = Access::default();
        // // access.read_all_components();
        // (world, access)

        unsafe {
            let index = world.get_resource_by_id(state.index_id).unwrap().deref();
            let asset = world.get_resource_by_id(state.asset_id).unwrap().deref();

            (world, index, asset)
        }
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
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
        _state: &Self::State,
        _filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        // assert!(
        //     filtered_access.access().is_compatible(state.access()),
        //     "DefRef conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        // );

        // filtered_access.access_mut().extend(state.access());
    }

    fn init_state(world: &mut World) -> Self::State {
        let asset_id = world.init_resource::<Assets<T::Asset>>();
        let index_id = world.init_resource::<DefIndex<T>>();
        DefQueryState { asset_id, index_id }
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        let asset_id = components.resource_id::<Assets<T::Asset>>()?;
        let index_id = components.resource_id::<DefIndex<T>>()?;
        Some(DefQueryState { asset_id, index_id })
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<'a, T: DefComponent> QueryData for DefEntityMut<'a, T> {
    const IS_READ_ONLY: bool = false;

    type ReadOnly = DefEntityRef<'a, T>;
    type Item<'w> = DefEntityMut<'w, T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        (world, index, asset): &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let entity = unsafe { debug_checked_unwrap_result(world.get_entity(entity)) };

        // SAFETY: mutable access to every component has been registered.
        DefEntityMut {
            entity,
            index,
            asset,
        }
    }
}
