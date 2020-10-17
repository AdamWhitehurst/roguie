use super::{Map, MonsterAI, Name, Position, Viewshed};
use rltk::{console, field_of_view, Point};
use specs::prelude::*;

pub struct MonsterAISystem {}

impl<'a> System<'a> for MonsterAISystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadExpect<'a, Point>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, MonsterAI>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, player_pt, mut viewsheds, mut ais, names, mut positions) = data;

        for (mut viewshed, ai, name, mut pos) in
            (&mut viewsheds, &mut ais, &names, &mut positions).join()
        {
            let distance =
                rltk::DistanceAlg::Pythagoras.distance2d(Point::new(pos.x, pos.y), *player_pt);

            if distance < 1.5 {
                // Attack goes here
                console::log(&format!("{} shouts insults", name.name));
                return;
            }

            if viewshed.visible_tiles.contains(&*player_pt) {
                ai.target_point = Some(*player_pt);
                console::log(&format!("{} shouts insults", name.name));
            }

            if let Some(pt) = ai.target_point {
                let path = rltk::a_star_search(
                    map.xy_idx(pos.x, pos.y) as i32,
                    map.xy_idx(pt.x, pt.y) as i32,
                    &mut *map,
                );
                if path.success && path.steps.len() > 1 {
                    pos.x = path.steps[1] as i32 % map.width;
                    pos.y = path.steps[1] as i32 / map.width;
                    viewshed.dirty = true;
                }
            }
        }
    }
}
