use super::{Map, MonsterAI, Position, RunState, Viewshed, WantsToMelee};
use rltk::{console, field_of_view, Point};
use specs::prelude::*;

pub struct MonsterAISystem {}

impl<'a> System<'a> for MonsterAISystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadExpect<'a, Point>,
        ReadExpect<'a, Entity>,
        Entities<'a>,
        ReadExpect<'a, RunState>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, MonsterAI>,
        WriteStorage<'a, WantsToMelee>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut map,
            player_pt,
            player_entity,
            entities,
            runstate,
            mut viewsheds,
            mut ais,
            mut wants_to_melee,
            mut positions,
        ) = data;

        if *runstate != RunState::MonsterTurn {
            return;
        }

        for (entity, mut viewshed, mut ai, mut pos) in
            (&entities, &mut viewsheds, &mut ais, &mut positions).join()
        {
            if viewshed.visible_tiles.contains(&*player_pt) {
                ai.target_point = Some(*player_pt);
            }

            if let Some(pt) = ai.target_point {
                let distance =
                    rltk::DistanceAlg::Pythagoras.distance2d(Point::new(pos.x, pos.y), pt);

                if distance < 1.5 {
                    wants_to_melee
                        .insert(
                            entity,
                            WantsToMelee {
                                target: *player_entity,
                            },
                        )
                        .expect("Unable to insert attack.");
                } else {
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
}
