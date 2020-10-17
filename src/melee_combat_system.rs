use super::{CombatStats, GameLog, Name, SufferDamage, WantsToMelee};
use rltk::console;
use specs::prelude::*;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
        WriteExpect<'a, GameLog>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut wants_melee, names, combat_stats, mut damage_store, mut log) = data;

        for (_entity, wants_melee, name, stats) in
            (&entities, &wants_melee, &names, &combat_stats).join()
        {
            if stats.hp > 0 {
                if let Some(target_stats) = combat_stats.get(wants_melee.target) {
                    if target_stats.hp > 0 {
                        if let Some(target_name) = names.get(wants_melee.target) {
                            let damage = i32::max(0, stats.power - target_stats.defense);

                            if damage == 0 {
                                log.entries.push(format!(
                                    "{} is unable to hurt {}",
                                    &name.name, &target_name.name
                                ));
                            } else {
                                log.entries.push(format!(
                                    "{} hits {}, for {} hp.",
                                    &name.name, &target_name.name, damage
                                ));
                                SufferDamage::new_damage(
                                    &mut damage_store,
                                    wants_melee.target,
                                    damage,
                                );
                            }
                        }
                    }
                }
            }
        }

        wants_melee.clear();
    }
}
