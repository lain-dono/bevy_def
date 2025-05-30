use crate::{DefComponent, DefIndex, DefMut, DefRef};
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    component::ComponentId,
    system::Res,
    world::{Mut, unsafe_world_cell::UnsafeEntityCell},
};
use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
};

pub struct DefEntityMut<'w, T: DefComponent> {
    entity: UnsafeEntityCell<'w>,
    asset: Res<'w, Assets<<T as DefComponent>::Asset>>,
    index: Res<'w, DefIndex<T>>,
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
