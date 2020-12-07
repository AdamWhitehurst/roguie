use super::common::*;
use super::{
    spawner, Map, MapBuilder, Position, Rect, TileType, MIN_ROOM_SIZE, SHOW_MAPGEN_VISUALIZER,
};
use rltk::RandomNumberGenerator;
use specs::prelude::*;
use std::collections::HashMap;

pub struct CellularAutomataBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
}

impl MapBuilder for CellularAutomataBuilder {
    fn get_map(&self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn build_map(&mut self) {
        self.build();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for area in self.noise_areas.iter() {
            spawner::fill_region(ecs, area.1, self.depth);
        }
    }

    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }
}

impl CellularAutomataBuilder {
    pub fn new(new_depth: i32) -> CellularAutomataBuilder {
        CellularAutomataBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        // Completely randomize the map, setting 55% of it to be floor.
        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let roll = rng.roll_dice(1, 100);
                let idx = self.map.xy_idx(x, y);
                if roll > 55 {
                    self.map.tiles[idx] = TileType::Floor
                } else {
                    self.map.tiles[idx] = TileType::Wall
                }
            }
        }
        self.take_snapshot();

        // Iteratively apply cellular automata rules
        for _i in 0..15 {
            let mut newtiles = self.map.tiles.clone();

            // For every tile
            for y in 1..self.map.height - 1 {
                for x in 1..self.map.width - 1 {
                    let idx = self.map.xy_idx(x, y);
                    let neighbors = self.dangerously_count_neighbors(idx);

                    // Crowd walls together or add a Wall in very open spaces
                    if neighbors > 4 || neighbors == 0 {
                        newtiles[idx] = TileType::Wall;
                    } else {
                        newtiles[idx] = TileType::Floor;
                    }
                }
            }

            self.map.tiles = newtiles.clone();
            self.take_snapshot();
        }

        // Clear out lonely pillars
        let mut newtiles = self.map.tiles.clone();
        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let idx = self.map.xy_idx(x, y);
                let neighbors = self.dangerously_count_neighbors(idx);

                if neighbors < 2 {
                    newtiles[idx] = TileType::Floor;
                }
            }
        }
        self.map.tiles = newtiles.clone();
        self.take_snapshot();

        // Find a starting point
        // Start at the middle and
        self.starting_position = Position {
            x: self.map.width / 2,
            y: self.map.height / 2,
        };
        let mut start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);

        //  Walk left until we find an open tile
        while self.map.tiles[start_idx] != TileType::Floor {
            self.starting_position.x -= 1;
            start_idx = self
                .map
                .xy_idx(self.starting_position.x, self.starting_position.y);
        }

        // Find all tiles we can reach from the starting point
        let map_starts: Vec<usize> = vec![start_idx];
        // http://www.roguebasin.com/index.php?title=The_Incredible_Power_of_Dijkstra_Maps
        // Get a map of distance values from map_starts
        let dijkstra_map = rltk::DijkstraMap::new(
            self.map.width,
            self.map.height,
            &map_starts,
            &self.map,
            200.0,
        );

        // (Index of exit tile, distance from start)
        let mut exit_tile = (0, 0.0f32);
        for (i, tile) in self.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance_to_start = dijkstra_map.map[i];
                // We can't get to this tile - so we'll make it a wall
                if distance_to_start == std::f32::MAX {
                    *tile = TileType::Wall;
                } else {
                    // If it is further away than our current exit candidate, move the exit
                    if distance_to_start > exit_tile.1 {
                        exit_tile.0 = i;
                        exit_tile.1 = distance_to_start;
                    }
                }
            }
        }
        self.take_snapshot();

        self.map.tiles[exit_tile.0] = TileType::DownStairs;
        self.take_snapshot();

        // Build a noise map for use in spawning entities later
        // Construct a Cellular Noise seed
        let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
        noise.set_noise_type(rltk::NoiseType::Cellular);
        noise.set_frequency(0.08);
        noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Manhattan);

        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let idx = self.map.xy_idx(x, y);
                if self.map.tiles[idx] == TileType::Floor {
                    let cell_value_f = noise.get_noise(x as f32, y as f32) * 10240.0;
                    let cell_value = cell_value_f as i32;

                    // Either add cell to existing area or insert as new
                    if self.noise_areas.contains_key(&cell_value) {
                        self.noise_areas.get_mut(&cell_value).unwrap().push(idx);
                    } else {
                        self.noise_areas.insert(cell_value, vec![idx]);
                    }
                }
            }
        }
    }

    /// Counts how many Wall neighbors passed `idx` has (both orthogonal and
    /// diagonal)
    ///
    /// !! WARNING: Does not check bounds !!
    fn dangerously_count_neighbors(&self, idx: usize) -> usize {
        let mut neighbors = 0;
        // x - 1
        if self.map.tiles[idx - 1] == TileType::Wall {
            neighbors += 1;
        }
        // x + 1
        if self.map.tiles[idx + 1] == TileType::Wall {
            neighbors += 1;
        }
        // y - 1
        if self.map.tiles[idx - self.map.width as usize] == TileType::Wall {
            neighbors += 1;
        }
        // y + 1
        if self.map.tiles[idx + self.map.width as usize] == TileType::Wall {
            neighbors += 1;
        }
        // x - 1, y - 1
        if self.map.tiles[idx - (self.map.width as usize - 1)] == TileType::Wall {
            neighbors += 1;
        }
        // x + 1, y - 1
        if self.map.tiles[idx - (self.map.width as usize + 1)] == TileType::Wall {
            neighbors += 1;
        }
        // x - 1, y + 1
        if self.map.tiles[idx + (self.map.width as usize - 1)] == TileType::Wall {
            neighbors += 1;
        }
        // x + 1, y + 1
        if self.map.tiles[idx + (self.map.width as usize + 1)] == TileType::Wall {
            neighbors += 1;
        }

        neighbors
    }
}
