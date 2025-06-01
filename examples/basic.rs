use bevy::{app::ScheduleRunnerPlugin, asset::weak_handle, log::LogPlugin, prelude::*};

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

    app.init_resource::<Status>();

    app.add_systems(Startup, startup);
    app.add_systems(Update, (show, spawn, increment_health));

    app.run();
}

#[derive(Resource, Default)]
struct Status {
    spawned: bool,
    handle: Option<Handle<StatAsset>>,
}

#[derive(Component)]
struct MarkerComponent;

const HEALTH: Handle<StatAsset> = weak_handle!("aa0c572f-1ebb-4f8b-b5c6-cfd8651799f2");

fn startup(mut stat_assets: ResMut<Assets<StatAsset>>, mut status: ResMut<Status>) {
    info!("startup");

    stat_assets.insert(
        &HEALTH,
        StatAsset {
            defname: String::from("health"),
            default: 35.0,
            minimal: 0.0,
            maximal: 100.0,
        },
    );

    let handle = stat_assets.add(StatAsset {
        defname: String::from("mana"),
        default: 0.7,
        minimal: 0.0,
        maximal: 0.9,
    });

    status.handle = Some(handle);
}

fn spawn(mut commands: Commands, mut status: ResMut<Status>, index: Res<DefIndex<Stat>>) {
    info!("spawn");

    if !status.spawned {
        let Some(health_id) = index.asset_to_id().get(&HEALTH.id()).copied() else {
            return;
        };

        let Some(mana_id) = index.find_by_name("mana").map(|(_, id)| id) else {
            return;
        };

        commands.spawn(MarkerComponent);

        commands
            .spawn(MarkerComponent)
            .queue(InsertDef::new(health_id, Stat { current: 35.0 }));

        commands
            .spawn(MarkerComponent)
            .queue(InsertDef::new(mana_id, Stat { current: 15.0 }));

        commands
            .spawn(MarkerComponent)
            .queue(InsertDef::new(health_id, Stat { current: 35.0 }))
            .queue(InsertDef::new(mana_id, Stat { current: 15.0 }));

        status.spawned = true;
    }
}

fn show(query: Query<(NameOrEntity, DefEntityRef<Stat>), With<MarkerComponent>>) {
    for (print, item) in query {
        let hp = item.get_ref(&HEALTH);
        let mp = item.find_ref("mana");

        if hp.is_none() && mp.is_none() {
            info!("{print} no hp, no mp");
        } else {
            if let Some(hp) = hp {
                info!(
                    "{print} hp: {} [{} .. {}]",
                    hp.value.current, hp.asset.minimal, hp.asset.maximal
                );
            }

            if let Some(mp) = mp {
                info!(
                    "{print} mp: {} [{} .. {}]",
                    mp.value.current, mp.asset.minimal, mp.asset.maximal
                );
            }
        }
    }
}

fn increment_health(query: Query<DefEntityMut<Stat>, With<MarkerComponent>>) {
    for mut stats in query {
        if let Some(mut hp) = stats.get_mut(&HEALTH) {
            hp.value.current += 0.2;
            hp.value.current = hp.value.current.clamp(hp.asset.minimal, hp.asset.maximal);
        }
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
