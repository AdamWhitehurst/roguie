#![allow(unused_variables)]
use rltk::{GameState, Point, Rltk};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

mod save_load_system;
pub use save_load_system::*;
mod monster_ai_system;
pub use monster_ai_system::*;
mod melee_combat_system;
pub use melee_combat_system::*;
mod damage_system;
pub use damage_system::*;
mod map_indexing_system;
pub use map_indexing_system::*;
mod components;
pub use components::*;
mod map;
pub use map::*;
mod player;
pub use player::*;
mod gui;
pub use gui::*;
mod gamelog;
pub use gamelog::*;
mod rect;
pub use rect::Rect;
mod visibility_system;
pub use visibility_system::*;
mod trigger_system;
pub use trigger_system::*;
mod spawner;
pub use spawner::*;
mod inventory_system;
pub use inventory_system::*;
mod random_table;
pub use random_table::*;
mod particle_system;
pub use particle_system::*;
mod hunger_system;
pub use hunger_system::*;
mod rex_assets;
pub use rex_assets::*;
pub mod map_builders;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    /// Systems have fully responded to latest player
    /// inputs and are now waiting for newer input
    AwaitingInput,
    /// Initial set up phase
    PreRun,
    /// Player has made new inputs and systems need
    /// to respond
    PlayerTurn,
    /// Systems have responded to latest player input
    /// and now ai (etc.) need to respond
    MonsterTurn,
    /// When user has their inventory screen open
    ShowInventory,
    /// When user has their drop-item screen open
    ShowDropItem,
    /// When user has to select a target for a spell
    ShowTargeting { range: i32, item: Entity },
    /// When user is in the main menu screen
    MainMenu {
        menu_selection: gui::MainMenuSelection,
    },
    /// Initiates saving game
    SaveGame,
    /// Initiates loading a new level
    NextLevel,
    /// Shows the Item removal menu
    ShowRemoveItem,
    /// Player has lost
    GameOver,
    /// Player has revealed the map
    MagicMapReveal { row: i32 },
}

pub struct State {
    pub ecs: World,
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        let mut newrunstate = *(self.ecs.fetch::<RunState>());

        ctx.cls();
        particle_system::cull_dead_particles(&mut self.ecs, ctx);
        // Handle drawing screen based on whether state is in-game or not
        match newrunstate {
            // Draw Main Menu screen
            RunState::MainMenu { .. } | RunState::GameOver { .. } => {}
            // Otherwise, handle drawing in-game map
            _ => {
                draw_map(&self.ecs, ctx);

                {
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let hidden = self.ecs.read_storage::<Hidden>();
                    let map = self.ecs.fetch::<Map>();

                    let mut data = (&positions, &renderables, !&hidden)
                        .join()
                        .collect::<Vec<_>>();
                    data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
                    for (pos, render, _) in data.iter() {
                        let idx = map.xy_idx(pos.x, pos.y);
                        if map.visible_tiles[idx] {
                            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph)
                        }
                    }

                    gui::draw_ui(&self.ecs, ctx);
                }
            }
        }

        // Handle updating state based on current state
        newrunstate = match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                RunState::AwaitingInput
            }

            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                match *self.ecs.fetch::<RunState>() {
                    RunState::MagicMapReveal { .. } => RunState::MagicMapReveal { row: 0 },
                    _ => RunState::MonsterTurn,
                }
            }

            RunState::MonsterTurn => {
                self.run_systems();
                RunState::AwaitingInput
            }

            RunState::AwaitingInput => player_input(self, ctx),

            RunState::ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => RunState::ShowInventory,
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = is_ranged.get(item_entity);
                        if let Some(ranged_item) = is_item_ranged {
                            RunState::ShowTargeting {
                                range: ranged_item.range,
                                item: item_entity,
                            }
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent
                                .insert(
                                    *self.ecs.fetch::<Entity>(),
                                    WantsToUseItem {
                                        item: item_entity,
                                        target: None,
                                    },
                                )
                                .expect("Unable to insert intent");

                            RunState::PlayerTurn
                        }
                    }
                }
            }

            RunState::NextLevel => {
                self.goto_next_level();
                RunState::PreRun
            }

            RunState::ShowDropItem => {
                let result = gui::drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => RunState::ShowDropItem,
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDropItem { item: item_entity },
                            )
                            .expect("Unable to insert intent");
                        RunState::PlayerTurn
                    }
                }
            }

            RunState::ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => newrunstate,
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToRemoveItem { item: item_entity },
                            )
                            .expect("Unable to insert intent");
                        RunState::PlayerTurn
                    }
                }
            }

            RunState::ShowTargeting { range, item } => {
                let (action, target) = gui::ranged_target(self, ctx, range);
                match action {
                    gui::ItemMenuResult::Cancel => RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => RunState::ShowTargeting { range, item },
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent
                            .insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item, target })
                            .expect("Unable to insert intent");

                        RunState::PlayerTurn
                    }
                }
            }

            RunState::MainMenu { .. } => {
                let result = gui::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection { selected } => RunState::MainMenu {
                        menu_selection: selected,
                    },
                    gui::MainMenuResult::Selected { selected } => match selected {
                        gui::MainMenuSelection::ResumeGame => RunState::PreRun,
                        gui::MainMenuSelection::NewGame => {
                            self.game_over_cleanup();
                            RunState::PreRun
                        }
                        gui::MainMenuSelection::SaveGame => RunState::SaveGame,
                        gui::MainMenuSelection::LoadGame => {
                            save_load_system::load_game(&mut self.ecs);
                            save_load_system::delete_save();
                            RunState::AwaitingInput
                        }
                        gui::MainMenuSelection::Quit => {
                            ::std::process::exit(0);
                        }
                    },
                }
            }

            RunState::SaveGame => {
                save_load_system::save_game(&mut self.ecs);

                RunState::MainMenu {
                    menu_selection: gui::MainMenuSelection::LoadGame,
                }
            }

            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::GameOverResult::NoSelection => newrunstate,
                    gui::GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();

                        RunState::MainMenu {
                            menu_selection: gui::MainMenuSelection::NewGame,
                        }
                    }
                }
            }

            RunState::MagicMapReveal { row } => {
                let mut map = self.ecs.fetch_mut::<Map>();
                for x in 0..MAP_WIDTH {
                    let idx = map.xy_idx(x as i32, row);
                    map.revealed_tiles[idx] = true;
                }
                if row as usize == MAP_HEIGHT - 1 {
                    RunState::MonsterTurn
                } else {
                    RunState::MagicMapReveal { row: row + 1 }
                }
            }
        };

        {
            // Set new runstate
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);
        let mut mob = MonsterAISystem {};
        mob.run_now(&self.ecs);
        // Triggers run after monster ai's update but before we apply
        // possible damage
        let mut triggers = TriggerSystem {};
        triggers.run_now(&self.ecs);
        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);
        let mut meleecombat = MeleeCombatSystem {};
        meleecombat.run_now(&self.ecs);
        let mut damagesystem = DamageSystem {};
        damagesystem.run_now(&self.ecs);
        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);
        let mut potions = ItemUseSystem {};
        potions.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);
        let mut item_remove = ItemRemoveSystem {};
        item_remove.run_now(&self.ecs);
        let mut hunger_system = HungerSystem {};
        hunger_system.run_now(&self.ecs);
        let mut particles = ParticleSpawnSystem {};
        particles.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn init_game(&mut self) {
        // Set up RunState resource
        self.ecs.insert(RunState::MainMenu {
            menu_selection: MainMenuSelection::NewGame,
        });

        // Build map and insertt resource
        let mut builder = map_builders::random_builder(1);
        builder.build_map();
        self.ecs.insert(builder.get_map());

        // Place player point resource
        let start = builder.get_starting_position();
        let (player_x, player_y) = (start.x, start.y);
        self.ecs.insert(Point::new(player_x, player_y));

        // Spawn player entity
        let player_entity = spawner::player(&mut self.ecs, player_x, player_y);
        self.ecs.insert(player_entity);

        // Init seed RNG
        let rng = rltk::RandomNumberGenerator::new();
        self.ecs.insert(rng);

        // Init the GameLog
        self.ecs.insert(GameLog {
            entries: vec!["Welcome to Roguie!".to_string()],
        });
    }

    /// Returns a vec of all Entities to delete. This includes non-players, and
    /// non-player-owned entities
    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete: Vec<Entity> = Vec::new();
        for entity in entities.join() {
            let mut should_delete = true;

            // Make sure not to delete player
            let p = player.get(entity);
            if let Some(_) = p {
                should_delete = false;
            }

            // Don't delete player's equipment
            let bp = backpack.get(entity);
            if let Some(bp) = bp {
                if bp.owner == *player_entity {
                    should_delete = false;
                }
            }

            let eq = equipped.get(entity);
            if let Some(eq) = eq {
                if eq.owner == *player_entity {
                    should_delete = false;
                }
            }

            if should_delete {
                to_delete.push(entity);
            }
        }
        to_delete
    }

    fn goto_next_level(&mut self) {
        // Delete entities that aren't the player or his/her equipment
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs
                .delete_entity(target)
                .expect("Unable to delete entity");
        }

        // Build a new map and place the player
        let mut builder;
        let current_depth;
        let player_start;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            current_depth = worldmap_resource.depth;
            builder = map_builders::random_builder(current_depth + 1);
            builder.build_map();
            *worldmap_resource = builder.get_map();
            player_start = builder.get_starting_position();
        }

        // Spawn bad guys
        builder.spawn_entities(&mut self.ecs);

        // Place the player and update resources
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_start.x, player_start.y);
        let mut position_components = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_pos_comp = position_components.get_mut(*player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_start.x;
            player_pos_comp.y = player_start.y;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }

        // Notify the player and give them some health
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog
            .entries
            .push("You descend to the next level, and take a moment to heal.".to_string());
        let mut player_health_store = self.ecs.write_storage::<CombatStats>();
        let player_health = player_health_store.get_mut(*player_entity);
        if let Some(player_health) = player_health {
            player_health.hp = i32::max(player_health.hp, player_health.max_hp / 2);
        }
    }

    fn game_over_cleanup(&mut self) {
        // Delete everything
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Deletion failed");
        }

        // Build a new map
        let mut builder;
        let current_depth;
        let player_start;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            current_depth = worldmap_resource.depth;
            builder = map_builders::random_builder(current_depth + 1);
            builder.build_map();
            *worldmap_resource = builder.get_map();
            player_start = builder.get_starting_position();
        }

        // Spawn bad guys
        builder.spawn_entities(&mut self.ecs);

        // Place the player and update resources
        let (player_x, player_y) = (player_start.x, player_start.y);
        let player_entity = spawner::player(&mut self.ecs, player_x, player_y);
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_x, player_y);
        let mut position_components = self.ecs.write_storage::<Position>();
        let mut player_entity_writer = self.ecs.write_resource::<Entity>();
        *player_entity_writer = player_entity;
        let player_pos_comp = position_components.get_mut(player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_x;
            player_pos_comp.y = player_y;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        // .with_automatic_console_resize(true)
        .with_title("Roguies: ")
        .build()?;
    let mut gs = State { ecs: World::new() };

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(rex_assets::RexAssets::new());

    // Registration order must match save/load order!
    gs.ecs.register::<Position>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<MonsterAI>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    gs.ecs.register::<WantsToRemoveItem>();
    gs.ecs.register::<HungerClock>();
    gs.ecs.register::<ProvidesFood>();
    gs.ecs.register::<ParticleLifetime>();
    gs.ecs.register::<MagicMapper>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntryTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();

    gs.init_game();

    rltk::main_loop(context, gs)
}
