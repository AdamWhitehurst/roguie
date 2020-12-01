use super::*;

mod common;
mod simple_map;

// use common::*;
use simple_map::*;

trait MapBuilder {
    fn build(new_depth: i32) -> Map;
}

pub fn build_random_map(new_depth: i32) -> Map {
    SimpleMapBuilder::build(new_depth)
}
