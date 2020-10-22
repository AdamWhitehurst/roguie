#![allow(unused_variables)]
use rltk::{GameState, Point, Rltk};
use specs::prelude::*;

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
pub use visibility_system::VisibilitySystem;
mod spawner;
pub use spawner::*;
mod inventory_system;
pub use inventory_system::*;

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
}

pub struct State {
    pub ecs: World,
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();
        let mut newrunstate = *(self.ecs.fetch::<RunState>());

        draw_map(&self.ecs, ctx);

        newrunstate = match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                RunState::AwaitingInput
            }
            RunState::PlayerTurn => {
                self.run_systems();
                RunState::MonsterTurn
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
        };

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);

        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let map = self.ecs.fetch::<Map>();

        let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
        data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));

        for (pos, render) in data.iter() {
            let idx = map.xy_idx(pos.x, pos.y);
            if map.visible_tiles[idx] {
                ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
            }
        }

        gui::draw_ui(&self.ecs, ctx);
    }
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);
        let mut mob = MonsterAISystem {};
        mob.run_now(&self.ecs);
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
        self.ecs.maintain();
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_automatic_console_resize(true)
        .with_title("Roguelike Tutorial")
        .build()?;
    let mut gs = State { ecs: World::new() };

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

    let map: Map = Map::new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center();
    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);
    gs.ecs.insert(player_entity);

    let rng = rltk::RandomNumberGenerator::new();
    gs.ecs.insert(rng);
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to Roguie".to_string()],
    });
    gs.ecs.insert(RunState::PreRun);
    gs.ecs.insert(Point::new(player_x, player_y));

    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }
    gs.ecs.insert(map);

    rltk::main_loop(context, gs)
}
