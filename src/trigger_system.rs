use super::{
    gamelog::GameLog, EntityMoved, EntryTrigger, Hidden, InflictsDamage, Map, Name,
    ParticleBuilder, Position, SingleActivation, SufferDamage,
};
use specs::prelude::*;

pub struct TriggerSystem {}

impl<'a> System<'a> for TriggerSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Map>,
        WriteStorage<'a, EntityMoved>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, EntryTrigger>,
        WriteStorage<'a, Hidden>,
        ReadStorage<'a, Name>,
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, InflictsDamage>,
        WriteExpect<'a, ParticleBuilder>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, SingleActivation>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            mut entity_moved,
            position,
            entry_trigger,
            mut hidden,
            names,
            entities,
            mut log,
            inflicts_damage,
            mut particle_builder,
            mut inflict_damage,
            single_activation,
        ) = data;

        // Save all entities that will be removed
        let mut remove_entities: Vec<Entity> = Vec::new();
        // Iterate the entities that moved and their final position
        for (entity, mut _entity_moved, pos) in (&entities, &mut entity_moved, &position).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            for entity_id in map.tile_content[idx].iter() {
                // Do not bother to check yourself for being a trap. (How could
                // a trap move? Maybe a future feature)
                if entity != *entity_id {
                    let maybe_trigger = entry_trigger.get(*entity_id);
                    match maybe_trigger {
                        None => {}
                        Some(_trigger) => {
                            // We triggered sumsum
                            let name = names.get(*entity_id);
                            if let Some(name) = name {
                                log.entries.push(format!("{} triggers!", &name.name));
                            }

                            // If the trap is damage inflicting, do it
                            let damage = inflicts_damage.get(*entity_id);
                            if let Some(damage) = damage {
                                particle_builder.request(
                                    pos.x,
                                    pos.y,
                                    rltk::RGB::named(rltk::ORANGE),
                                    rltk::RGB::named(rltk::BLACK),
                                    rltk::to_cp437('‼'),
                                    200.0,
                                );
                                SufferDamage::new_damage(
                                    &mut inflict_damage,
                                    entity,
                                    damage.damage,
                                );
                            }

                            // If it is single activation, it needs to be removed
                            let sa = single_activation.get(*entity_id);
                            if let Some(_sa) = sa {
                                remove_entities.push(*entity_id);
                            }

                            // The trap is no longer hidden
                            hidden.remove(*entity_id);
                        }
                    }
                }
            }
        }

        // Remove any single activation traps
        for trap in remove_entities.iter() {
            entities
                .delete(*trap)
                .expect("Unable to delete trigger entity");
        }

        // Remove all entity movement markers
        entity_moved.clear();
    }
}
