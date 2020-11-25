use super::{
    gamelog::GameLog, particle_system::ParticleBuilder, CombatStats, DefenseBonus, Equipped,
    MeleePowerBonus, Name, Position, SufferDamage, WantsToMelee,
};
use specs::prelude::*;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    type SystemData = (
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, MeleePowerBonus>,
        ReadStorage<'a, DefenseBonus>,
        ReadStorage<'a, Equipped>,
        WriteExpect<'a, ParticleBuilder>,
        ReadStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut log,
            mut wants_melee,
            names,
            combat_stats,
            mut inflict_damage,
            melee_power_bonuses,
            defense_bonuses,
            equipped,
            mut particle_builder,
            positions,
        ) = data;
        for (entity, wants_melee, name, stats) in
            (&entities, &wants_melee, &names, &combat_stats).join()
        {
            // Attacking entity must be alive
            if stats.hp > 0 {
                // Add any melee powe bonuses
                let mut offensive_bonus = 0;
                for (_item_entity, power_bonus, equipped_by) in
                    (&entities, &melee_power_bonuses, &equipped).join()
                // .filter(|b| b.2.owner == entity)
                {
                    // Find any equipped items that give a melee power bonus w/
                    // and owner of this entity
                    if equipped_by.owner == entity {
                        offensive_bonus += power_bonus.power;
                    }
                }

                // Targetted entity must have combat stats (health)
                if let Some(target_stats) = combat_stats.get(wants_melee.target) {
                    // Defending entity must be alive
                    if target_stats.hp > 0 {
                        // Target must have a name
                        if let Some(target_name) = names.get(wants_melee.target) {
                            // Determine any defensive bonuses
                            let mut defensive_bonus = 0;
                            for (_item_entity, defense_bonus, equipped_by) in
                                (&entities, &defense_bonuses, &equipped).join()
                            {
                                // Find any equipped items w/ the target entity
                                // as owner that has a defense bonus comp
                                if equipped_by.owner == wants_melee.target {
                                    defensive_bonus += defense_bonus.defense;
                                }
                            }

                            let pos = positions.get(wants_melee.target);

                            // ADd a particle effect to the targets position
                            if let Some(pos) = pos {
                                particle_builder.request(
                                    pos.x,
                                    pos.y,
                                    rltk::RGB::named(rltk::ORANGE),
                                    rltk::RGB::named(rltk::BLACK),
                                    rltk::to_cp437('‼'),
                                    200.0,
                                );
                            }

                            let damage = i32::max(
                                0,
                                (stats.power + offensive_bonus)
                                    - (target_stats.defense + defensive_bonus),
                            );

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
                                    &mut inflict_damage,
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
