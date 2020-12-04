use crate::{gamelog::GameLog, Hidden, Map, Name, Player, Position, RevealChance, Viewshed};
use rltk::{field_of_view, Point};
use specs::prelude::*;

pub struct VisibilitySystem {}

impl<'a> System<'a> for VisibilitySystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        Entities<'a>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Hidden>,
        WriteExpect<'a, rltk::RandomNumberGenerator>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, RevealChance>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut map,
            entities,
            mut viewshed,
            pos,
            player,
            mut hidden,
            mut rng,
            mut log,
            names,
            reveal_chances,
        ) = data;

        for (ent, viewshed, pos) in (&entities, &mut viewshed, &pos).join() {
            if viewshed.dirty {
                viewshed.dirty = false;
                viewshed.visible_tiles.clear();
                viewshed.visible_tiles =
                    field_of_view(Point::new(pos.x, pos.y), viewshed.range, &*map);
                viewshed
                    .visible_tiles
                    .retain(|p| p.x >= 0 && p.x < map.width && p.y >= 0 && p.y < map.height);

                // If this is the player, reveal what they can see
                let _p: Option<&Player> = player.get(ent);
                if let Some(_p) = _p {
                    for t in map.visible_tiles.iter_mut() {
                        *t = false
                    }
                    for vis in viewshed.visible_tiles.iter() {
                        for vis in viewshed.visible_tiles.iter() {
                            let idx = map.xy_idx(vis.x, vis.y);
                            map.revealed_tiles[idx] = true;
                            map.visible_tiles[idx] = true;

                            // Try to reveal things that have a chance
                            for e in map.tile_content[idx].iter() {
                                let maybe_hidden = hidden.get(*e);
                                let maybe_reveal_chance = reveal_chances.get(*e);
                                if let (Some(_), Some(reveal_chance)) =
                                    (maybe_hidden, maybe_reveal_chance)
                                {
                                    if rng.roll_dice(1, reveal_chance.chance) == 1 {
                                        let name = names.get(*e);
                                        if let Some(name) = name {
                                            log.entries
                                                .push(format!("You spotted a {}.", &name.name));
                                        }
                                        hidden.remove(*e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
