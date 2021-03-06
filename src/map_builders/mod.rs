use super::*;

mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod common;
mod simple_map;

// use common::*;
use bsp_dungeon::*;
use bsp_interior::*;
use cellular_automata::*;
use simple_map::*;

const MIN_ROOM_SIZE: i32 = 8;

pub trait MapBuilder {
    fn build_map(&mut self);
    fn spawn_entities(&mut self, ecs: &mut World);
    fn get_map(&self) -> Map;
    fn get_starting_position(&self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    let mut rng = rltk::RandomNumberGenerator::new();
    let builder = rng.roll_dice(1, 7);
    match builder {
        1 => Box::new(BspDungeonBuilder::new(new_depth)),
        2 => Box::new(BspInteriorBuilder::new(new_depth)),
        3 => Box::new(SimpleMapBuilder::new(new_depth)),
        _ => Box::new(CellularAutomataBuilder::new(new_depth)),
    }
}
