use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, AssetEvent, AssetEvents, AssetId, Assets};
use bevy_ecs::{
    component::{
        ComponentCloneBehavior, ComponentDescriptor, ComponentHook, ComponentId, StorageType,
    },
    event::{EventCursor, Events},
    query::Access,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{EntityCommand, Local, Res, SystemParam},
    world::{EntityWorldMut, FilteredEntityMut, FilteredEntityRef, Mut, World},
};
use bevy_log::{error, info, warn};
use bevy_platform::collections::HashMap;
use bevy_ptr::OwningPtr;
use std::{
    alloc::Layout,
    borrow::{Borrow, Cow},
    hash::Hash,
    marker::PhantomData,
    mem::needs_drop,
};

mod debug;
mod entity_mut;
mod entity_ref;

pub use self::entity_mut::DefEntityMut;
pub use self::entity_ref::DefEntityRef;

pub unsafe trait DefComponent: Send + Sync + 'static {
    /// Asset attached to a component.
    type Asset: Asset;

    /// The storage used for a specific component type.
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// Gets the name of the [`Component`] from the asset.
    fn defname(asset: &Self::Asset) -> Cow<'static, str>;

    /// Gets the `on_add` [`ComponentHook`] for this [`DefComponent`] if one is defined.
    fn on_add() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_insert` [`ComponentHook`] for this [`DefComponent`] if one is defined.
    fn on_insert() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_replace` [`ComponentHook`] for this [`DefComponent`] if one is defined.
    fn on_replace() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_remove` [`ComponentHook`] for this [`DefComponent`] if one is defined.
    fn on_remove() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_despawn` [`ComponentHook`] for this [`DefComponent`] if one is defined.
    fn on_despawn() -> Option<ComponentHook> {
        None
    }

    // TODO: map_entities, maybe ComponentCloneBehavior
}

pub struct DefRef<'value, 'asset, T: DefComponent> {
    pub value: &'value T,
    pub asset: &'asset T::Asset,
}

pub struct DefMut<'value, 'asset, T: DefComponent> {
    pub value: Mut<'value, T>,
    pub asset: &'asset T::Asset,
}

pub struct DefPlugin<T: DefComponent>(PhantomData<fn() -> T>);

impl<T: DefComponent> Default for DefPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: DefComponent> Plugin for DefPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<DefIndex<T>>();
        app.init_asset::<T::Asset>();
        app.add_systems(PostUpdate, def_maintain_system::<T>.after(AssetEvents));

        let world = app.world_mut();
        let index_id = world.resource_id::<DefIndex<T>>().unwrap();
        let asset_id = world.resource_id::<Assets<T::Asset>>().unwrap();

        let mut index = world.resource_mut::<DefIndex<T>>();

        index.access_ref.add_resource_read(index_id);
        index.access_ref.add_resource_read(asset_id);

        index.access_mut.add_resource_read(index_id);
        index.access_mut.add_resource_read(asset_id);
    }
}

pub fn def_maintain_system<T: DefComponent>(
    world: &mut World,
    mut reader: Local<EventCursor<AssetEvent<T::Asset>>>,
) {
    world.resource_scope(|world, events: Mut<Events<AssetEvent<T::Asset>>>| {
        world.resource_scope(|world, mut def_index: Mut<DefIndex<T>>| {
            for event in reader.read(&events) {
                match event {
                    &AssetEvent::Added { id } => {
                        info!("added {id}");
                        def_index.register(world, id);
                    }
                    AssetEvent::Modified { id } => info!("modified {id}"),
                    AssetEvent::Removed { id } => error!("removed {id}"),
                    AssetEvent::Unused { id } => error!("unused {id}"),
                    AssetEvent::LoadedWithDependencies { id } => {
                        warn!("full loaded {id}")
                    }
                }
            }
        });
    });
}

pub struct InsertDef<T: DefComponent> {
    component_id: ComponentId,
    value: T,
}

impl<T: DefComponent> InsertDef<T> {
    pub const fn new(component_id: ComponentId, value: T) -> Self {
        Self {
            component_id,
            value,
        }
    }
}

impl<T: DefComponent> EntityCommand for InsertDef<T> {
    fn apply(self, mut entity: EntityWorldMut<'_>) {
        OwningPtr::make(self.value, |component| unsafe {
            entity.insert_by_id(self.component_id, component);
        });
    }
}

pub struct RemoveDef<T: DefComponent> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<T: DefComponent> RemoveDef<T> {
    pub const fn new(component_id: ComponentId) -> Self {
        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<T: DefComponent> EntityCommand for RemoveDef<T> {
    fn apply(self, mut entity: EntityWorldMut<'_>) {
        entity.remove_by_id(self.component_id);
    }
}

#[derive(Resource)]
pub struct DefIndex<T: DefComponent> {
    names: HashMap<Cow<'static, str>, (AssetId<T::Asset>, ComponentId)>,

    asset_to_id: HashMap<AssetId<T::Asset>, ComponentId>,
    id_to_asset: HashMap<ComponentId, AssetId<T::Asset>>,

    access_ref: Access<ComponentId>,
    access_mut: Access<ComponentId>,

    marker: PhantomData<fn() -> T>,
}

impl<T: DefComponent> Default for DefIndex<T> {
    fn default() -> Self {
        Self {
            names: HashMap::default(),

            asset_to_id: HashMap::default(),
            id_to_asset: HashMap::default(),

            access_ref: Access::default(),
            access_mut: Access::default(),

            marker: PhantomData,
        }
    }
}

impl<T: DefComponent> DefIndex<T> {
    fn register(&mut self, world: &mut World, id: impl Into<AssetId<T::Asset>>) {
        let id = id.into();
        let name = T::defname(world.resource_mut::<Assets<T::Asset>>().get(id).unwrap());
        self.names.entry(name.clone()).or_insert_with(|| {
            let component_id = world.register_component_with_descriptor(unsafe {
                let layout = Layout::new::<T>();
                let storage = T::STORAGE_TYPE;
                let drop = needs_drop::<T>().then_some(Self::drop_ptr as _);
                let clone = ComponentCloneBehavior::Default;
                ComponentDescriptor::new_with_layout(name, storage, layout, drop, true, clone)
            });

            self.asset_to_id.insert(id, component_id);
            self.id_to_asset.insert(component_id, id);

            self.access_ref.add_component_read(component_id);
            self.access_mut.add_component_write(component_id);

            let hooks = world.register_component_hooks_by_id(component_id).unwrap();

            if let Some(hook) = T::on_add() {
                hooks.on_add(hook);
            }
            if let Some(hook) = T::on_insert() {
                hooks.on_insert(hook);
            }
            if let Some(hook) = T::on_replace() {
                hooks.on_replace(hook);
            }
            if let Some(hook) = T::on_remove() {
                hooks.on_remove(hook);
            }
            if let Some(hook) = T::on_despawn() {
                hooks.on_despawn(hook);
            }

            (id, component_id)
        });
    }

    pub fn find_by_name<Q>(&self, name: &Q) -> Option<(AssetId<T::Asset>, ComponentId)>
    where
        Q: Hash + Eq + ?Sized,
        Cow<'static, str>: Borrow<Q>,
    {
        self.names.get(name).copied()
    }

    pub fn names(&self) -> &HashMap<Cow<'static, str>, (AssetId<T::Asset>, ComponentId)> {
        &self.names
    }

    pub fn asset_to_id(&self) -> &HashMap<AssetId<T::Asset>, ComponentId> {
        &self.asset_to_id
    }

    pub fn id_to_asset(&self) -> &HashMap<ComponentId, AssetId<T::Asset>> {
        &self.id_to_asset
    }

    pub fn access_ref(&self) -> Access<ComponentId> {
        self.access_ref.clone()
    }

    pub fn access_mut(&self) -> Access<ComponentId> {
        self.access_mut.clone()
    }

    unsafe fn drop_ptr(x: OwningPtr<'_>) {
        // SAFETY: Contract is required to be upheld by the caller.
        unsafe { x.drop_as::<T>() }
    }
}

#[derive(SystemParam)]
pub struct DefParam<'w, T: DefComponent> {
    pub asset: Res<'w, Assets<<T as DefComponent>::Asset>>,
    pub index: Res<'w, DefIndex<T>>,
}

impl<'w, T: DefComponent> DefParam<'w, T> {
    pub fn asset(&self, id: impl Into<AssetId<T::Asset>>) -> Option<(ComponentId, &'_ T::Asset)> {
        let asset_index = id.into();
        let asset = self.asset.get(asset_index)?;
        let component_id = self.index.asset_to_id.get(&asset_index).copied()?;
        Some((component_id, asset))
    }

    pub fn filtered_entity_ref<'a>(
        &self,
        entity: &'a FilteredEntityRef<'w>,
        id: impl Into<AssetId<T::Asset>>,
    ) -> Option<DefRef<'a, '_, T>> {
        let (component_id, asset) = self.asset(id)?;
        let value = unsafe { entity.get_by_id(component_id)?.deref() };
        Some(DefRef { value, asset })
    }

    pub fn filtered_entity_mut<'a>(
        &self,
        entity: &'a mut FilteredEntityMut<'w>,
        id: impl Into<AssetId<T::Asset>>,
    ) -> Option<DefMut<'a, '_, T>> {
        let (component_id, asset) = self.asset(id)?;
        let value = unsafe { entity.get_mut_by_id(component_id)?.with_type::<T>() };
        Some(DefMut { value, asset })
    }
}
