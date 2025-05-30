use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, platform::collections::HashMap, prelude::*};

use bevy_def::*;
use std::time::Duration;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(200))),
        LogPlugin::default(),
        AssetPlugin::default(),
        DefPlugin::<Stat>::default(),
    ));

    app.init_resource::<Stats>();

    app.add_systems(Startup, startup);
    app.add_systems(Update, spawn);
    app.add_systems(Update, show);

    app.run();
}

#[derive(Resource, Default)]
struct Stats {
    all: Vec<Handle<StatAsset>>,
    spawned: HashMap<AssetId<StatAsset>, Entity>,
}

fn startup(mut stat_assets: ResMut<Assets<StatAsset>>, mut stats: ResMut<Stats>) {
    info!("startup");

    stats.all.push(stat_assets.add(StatAsset {
        defname: String::from("health"),
        default: 0.35,
        minimal: 0.01,
        maximal: 0.50,
    }));
}

fn spawn(mut commands: Commands, mut stats: ResMut<Stats>, index: Res<DefIndex<Stat>>) {
    info!("spawn");
    let stats: &mut Stats = &mut stats;

    for handle in &stats.all {
        if !stats.spawned.contains_key(&handle.id()) {
            if let Some((asset_id, component_id)) = index.find_by_name("health") {
                let mut entity = commands.spawn_empty();
                entity.queue(InsertDef::new(component_id, Stat { current: 5.0 }));
                stats.spawned.insert(asset_id, entity.id());
            }
        }
    }
}

fn show(query: Query<DefEntityRef<Stat>>) {
    info!("show");

    for item in query {
        let health = item.find_ref("health").unwrap();

        info!(
            "{}: {} [{} .. {}]",
            health.asset.defname, health.value.current, health.asset.minimal, health.asset.maximal
        );
    }
}

#[derive(Reflect, Debug)]
pub struct Stat {
    pub current: f32,
}

unsafe impl DefComponent for Stat {
    type Asset = StatAsset;

    fn defname(asset: &Self::Asset) -> std::borrow::Cow<'static, str> {
        asset.defname.clone().into()
    }
}

#[derive(Asset, Reflect, Debug)]
pub struct StatAsset {
    pub defname: String,
    pub default: f32,
    pub minimal: f32,
    pub maximal: f32,
}
