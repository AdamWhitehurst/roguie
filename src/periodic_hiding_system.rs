use crate::{EntryTrigger, Hidden, Name, PeriodicHiding};
use specs::prelude::*;

pub struct PeriodicHidingSystem {}

impl<'a> System<'a> for PeriodicHidingSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, PeriodicHiding>,
        WriteStorage<'a, Hidden>,
        WriteStorage<'a, EntryTrigger>,
        ReadStorage<'a, Name>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut periodic_hiding_store, mut hidden_store, mut trigger_store, names) =
            data;

        for (e, hiding) in (&entities, &mut periodic_hiding_store).join() {
            hiding.offset = (hiding.offset + 1) % hiding.period;
            if hiding.offset == 0 {
                if let Some(hidden) = hidden_store.get(e) {
                    hidden_store.remove(e);
                    trigger_store
                        .insert(e, EntryTrigger {})
                        .expect("Unable to insert EntryTrigger in Periodic Hiding System");
                } else {
                    trigger_store.remove(e);
                    hidden_store
                        .insert(e, Hidden {})
                        .expect("Unable to insert Hidden in Periodic Hiding System");
                }
            }
        }
    }
}
