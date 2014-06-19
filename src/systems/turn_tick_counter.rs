use components::{Side, Turn};
use ecm::{ComponentManager, ECM, Entity};


define_system! {
    name: TurnTickCounterSystem;
    components(Turn);
    resources(ecm: ECM, side: Side);
    fn process_entity(&mut self, _dt_ms: uint, e: Entity) {
        let ecm = &mut *self.ecm();
        let turn: Turn = ecm.get(e);
        if turn.side == *self.side() {
            ecm.set(e, Turn{spent_this_tick: 0, .. turn});
        }
    }
}
