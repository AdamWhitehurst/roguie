use super::*;
use specs::error::NoError;
use specs::saveload::{DeserializeComponents, MarkedBuilder, SerializeComponents};
use std::fs::{read_to_string, File};
use std::path::Path;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    fn save(s: &[u8]);

    #[wasm_bindgen(catch)]
    fn load() -> std::result::Result<JsValue, JsValue>;
}

/// Helper macro for serializing stores of Components to be saved
macro_rules! serialize_individually {
    ($ecs:expr, $ser:expr, $data:expr, $( $type:ty),*) => {
        $(
        SerializeComponents::<NoError, SimpleMarker<SerializeMe>>::serialize(
            &( $ecs.read_storage::<$type>(), ),
            &$data.0,
            &$data.1,
            &mut $ser,
        )
        .unwrap();
        )*
    };
}

/// Helper macro for deserializing components stores from saved files
macro_rules! deserialize_individually {
    ($ecs:expr, $de:expr, $data:expr, $( $type:ty),*) => {
        $(
        DeserializeComponents::<NoError, _>::deserialize(
            &mut ( &mut $ecs.write_storage::<$type>(), ),
            &mut $data.0, // entities
            &mut $data.1, // marker
            &mut $data.2, // allocater
            &mut $de,
        )
        .unwrap();
        )*
    };
}

fn deserialize_world<'de, R>(ecs: &mut World, deserializer: &'de mut serde_json::Deserializer<R>)
where
    R: serde_json::de::Read<'de>,
{
    let mut d = (
        &mut ecs.entities(),
        &mut ecs.write_storage::<SimpleMarker<SerializeMe>>(),
        &mut ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>(),
    );

    // Order of components must match macro in save_game
    deserialize_individually!(
        ecs,
        *deserializer,
        d,
        Position,
        Renderable,
        Player,
        Viewshed,
        MonsterAI,
        Name,
        BlocksTile,
        CombatStats,
        SufferDamage,
        WantsToMelee,
        Item,
        Consumable,
        Ranged,
        InflictsDamage,
        AreaOfEffect,
        Confusion,
        ProvidesHealing,
        InBackpack,
        WantsToPickupItem,
        WantsToUseItem,
        WantsToDropItem,
        SerializationHelper,
        Equippable,
        Equipped,
        MeleePowerBonus,
        DefenseBonus,
        WantsToRemoveItem,
        ParticleLifetime,
        HungerClock,
        ProvidesFood,
        MagicMapper,
        Hidden,
        EntryTrigger,
        EntityMoved,
        SingleActivation
    );
}

fn serialize_world<W, F>(ecs: &mut World, serializer: &mut serde_json::Serializer<W, F>)
where
    W: std::io::Write,
    F: serde_json::ser::Formatter,
{
    let data = (
        ecs.entities(),
        ecs.read_storage::<SimpleMarker<SerializeMe>>(),
    );

    // Order of components must match macro in load_game
    serialize_individually!(
        ecs,
        *serializer,
        data,
        Position,
        Renderable,
        Player,
        Viewshed,
        MonsterAI,
        Name,
        BlocksTile,
        CombatStats,
        SufferDamage,
        WantsToMelee,
        Item,
        Consumable,
        Ranged,
        InflictsDamage,
        AreaOfEffect,
        Confusion,
        ProvidesHealing,
        InBackpack,
        WantsToPickupItem,
        WantsToUseItem,
        WantsToDropItem,
        SerializationHelper,
        Equippable,
        Equipped,
        MeleePowerBonus,
        DefenseBonus,
        WantsToRemoveItem,
        ParticleLifetime,
        HungerClock,
        ProvidesFood,
        MagicMapper,
        Hidden,
        EntryTrigger,
        EntityMoved,
        SingleActivation
    );
}

#[cfg(target_arch = "wasm32")]
pub fn save_game(ecs: &mut World) {
    // Create helper
    let mapcopy = ecs.get_mut::<super::map::Map>().unwrap().clone();
    let savehelper = ecs
        .create_entity()
        .with(SerializationHelper { map: mapcopy })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    // Serialization
    {
        let writer = Vec::new();
        let mut serializer = serde_json::Serializer::new(writer);

        serialize_world(ecs, &mut serializer);

        let output = std::str::from_utf8(serializer.into_inner().as_slice())
            .unwrap()
            .to_string();
        let window: web_sys::Window = web_sys::window().expect("no global window");
        match window.local_storage() {
            Ok(store) => {
                if let Some(store) = store {
                    let res = store.set_item("save", output.as_str());
                    match res {
                        _ => {}
                    }
                }
            }

            Err(_) => {}
        }
    }

    // Clean up
    ecs.delete_entity(savehelper).expect("Crash on cleanup");
}

/// Saves a new game file
#[cfg(not(target_arch = "wasm32"))]
pub fn save_game(ecs: &mut World) {
    // Create helper
    let mapcopy = ecs.get_mut::<super::map::Map>().unwrap().clone();
    let savehelper = ecs
        .create_entity()
        .with(SerializationHelper { map: mapcopy })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    // Serialization
    {
        let writer = File::create("./savegame.json").unwrap();
        let mut serializer = serde_json::Serializer::new(writer);

        serialize_world(ecs, &mut serializer);
    }

    // Clean up
    ecs.delete_entity(savehelper).expect("Crash on cleanup");
}

/// Checks if a save file exists
#[cfg(not(target_arch = "wasm32"))]
pub fn does_save_exist() -> bool {
    Path::new("./savegame.json").exists()
}

/// Checks if a save file exists
#[cfg(target_arch = "wasm32")]
pub fn does_save_exist() -> bool {
    let mut found = false;
    let window: web_sys::Window = web_sys::window().expect("no global window");
    match window.local_storage() {
        Ok(store) => {
            if let Some(store) = store {
                match store.get_item("save") {
                    Ok(data) => {
                        if let Some(save) = data {
                            found = true;
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        Err(_) => {}
    }

    return found;
}

/// Whether the game can be quit or not
/// (on wasm architectures, game would crash if
/// user tries to quit)
#[cfg(target_arch = "wasm32")]
pub fn can_quit_game() -> bool {
    false
}

/// Whether the game can be quit or not
/// (on wasm architectures, game would crash if
/// user tries to quit)
#[cfg(not(target_arch = "wasm32"))]
pub fn can_quit_game() -> bool {
    true
}

/// Loads a saved game file, assuming there is one
#[cfg(target_arch = "wasm32")]
pub fn load_game(ecs: &mut World) {
    let mut opt_save_data: Option<String> = None;
    let window: web_sys::Window = web_sys::window().expect("no global window");
    match window.local_storage() {
        Ok(store) => {
            if let Some(store) = store {
                match store.get_item("save") {
                    Ok(data) => {
                        if let Some(save) = data {
                            opt_save_data = Some(save);
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        Err(_) => {}
    }

    if let Some(save_data_string) = opt_save_data {
        {
            // Delete everything in two steps to avoid
            // invalidation the iterator in the first pass
            let mut to_delete = Vec::new();
            for e in ecs.entities().join() {
                to_delete.push(e);
            }

            for del in to_delete.iter() {
                ecs.delete_entity(*del)
                    .expect("load_game Entity Deletion Failed.");
            }
        }

        let mut deserializer = serde_json::Deserializer::from_str(&save_data_string);

        deserialize_world(ecs, &mut deserializer);

        let mut deleteme: Option<Entity> = None;
        {
            let entities = ecs.entities();
            let helper = ecs.read_storage::<SerializationHelper>();
            let player = ecs.read_storage::<Player>();
            let position = ecs.read_storage::<Position>();
            for (e, h) in (&entities, &helper).join() {
                let mut worldmap = ecs.write_resource::<super::map::Map>();

                *worldmap = h.map.clone();
                worldmap.tile_content = vec![Vec::new(); super::map::MAP_COUNT];
                deleteme = Some(e);
            }

            for (e, _p, pos) in (&entities, &player, &position).join() {
                let mut ppos = ecs.write_resource::<rltk::Point>();
                *ppos = rltk::Point::new(pos.x, pos.y);
                let mut player_resource = ecs.write_resource::<Entity>();
                *player_resource = e;
            }
        }

        if let Some(e) = deleteme {
            ecs.delete_entity(e)
                .expect("load_game Unable to delete helper");
        }
    }
}

/// Loads a saved game file, assuming there is one
#[cfg(not(target_arch = "wasm32"))]
pub fn load_game(ecs: &mut World) {
    {
        // Delete everything in two steps to avoid
        // invalidation the iterator in the first pass
        let mut to_delete = Vec::new();
        for e in ecs.entities().join() {
            to_delete.push(e);
        }

        for del in to_delete.iter() {
            ecs.delete_entity(*del)
                .expect("load_game Entity Deletion Failed.");
        }
    }

    let data = read_to_string("./savegame.json").unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&data);

    deserialize_world(ecs, &mut deserializer);

    let mut deleteme: Option<Entity> = None;
    {
        let entities = ecs.entities();
        let helper = ecs.read_storage::<SerializationHelper>();
        let player = ecs.read_storage::<Player>();
        let position = ecs.read_storage::<Position>();
        for (e, h) in (&entities, &helper).join() {
            let mut worldmap = ecs.write_resource::<super::map::Map>();

            *worldmap = h.map.clone();
            worldmap.tile_content = vec![Vec::new(); super::map::MAP_COUNT];
            deleteme = Some(e);
        }

        for (e, _p, pos) in (&entities, &player, &position).join() {
            let mut ppos = ecs.write_resource::<rltk::Point>();
            *ppos = rltk::Point::new(pos.x, pos.y);
            let mut player_resource = ecs.write_resource::<Entity>();
            *player_resource = e;
        }
    }

    if let Some(e) = deleteme {
        ecs.delete_entity(e)
            .expect("load_game Unable to delete helper");
    }
}

/// Deletes the save file
#[cfg(not(target_arch = "wasm32"))]
pub fn delete_save() {
    if Path::new("./savegame.json").exists() {
        std::fs::remove_file("./savegame.json").expect("Unable to delete file");
    }
}
/// Deletes the save file
#[cfg(target_arch = "wasm32")]
pub fn delete_save() {
    if !does_save_exist() {
        return;
    }

    let window: web_sys::Window = web_sys::window().expect("no global window");
    match window.local_storage() {
        Ok(store) => {
            if let Some(store) = store {
                match store.remove_item("save") {
                    _ => {}
                }
            }
        }

        Err(_) => {}
    }
}
