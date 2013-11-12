use components::*;
use map::{Walkable, Solid};
use super::ai;
use super::super::Resources;

pub fn system(e: ID,
              ecm: &mut ComponentManager,
              res: &mut Resources) {
    ensure_components!(ecm, e, Position, Destination, Turn);
    let turn = ecm.get_turn(e);
    if turn.ap <= 0 {return}

    let pos = ecm.get_position(e);
    let dest = ecm.get_destination(e);
    if (pos.x, pos.y) == (dest.x, dest.y) {
        // Wait (spends an AP but do nothing)
        println!("Entity {} waits.", *e);
        ecm.set_turn(e, turn.spend_ap(1));
        ecm.remove_destination(e);
    } else if ai::distance(&pos, &Position{x: dest.x, y: dest.y}) == 1 {
        if res.map.is_walkable((dest.x, dest.y))  {  // Move to the cell
            ecm.set_turn(e, turn.spend_ap(1));
            { // Update both the entity position component and the map:
                res.map.move_entity(*e, (pos.x, pos.y), (dest.x, dest.y));
                ecm.set_position(e, Position{x: dest.x, y: dest.y});
            }
            ecm.remove_destination(e);
        } else {  // Bump into the blocked entity
            // TODO: assert there's only one solid entity on pos [x, y]
            for (bumpee, walkable) in res.map.entities_on_pos((dest.x, dest.y)) {
                assert!(bumpee != *e);
                match walkable {
                    Walkable => loop,
                    Solid => {
                        println!("Entity {} bumped into {} at: ({}, {})",
                                 *e, bumpee, dest.x, dest.y);
                        ecm.set_bump(e, Bump(ID(bumpee)));
                        ecm.remove_destination(e);
                        break;
                    }
                }
            }
        }
    } else {  // Farther away than 1 space. Need to use path finding
        match res.map.find_path((pos.x, pos.y), (dest.x, dest.y)) {
            Some(ref mut path) => {
                assert!(path.len() > 1,
                        "The path shouldn't be trivial. We already handled that.");
                match path.walk() {
                    Some((x, y)) => {
                        let new_pos = Position{x: x, y: y};
                        assert!(ai::distance(&pos, &new_pos) == 1,
                                "The step should be right next to the curret pos.");
                        ecm.set_turn(e, turn.spend_ap(1));
                        { // Update both the entity position component and the map:
                            res.map.move_entity(*e, (pos.x, pos.y), (x, y));
                            ecm.set_position(e, new_pos);
                        }
                    }
                    // "The path exists but can't be walked?!"
                    None => unreachable!(),
                }
            }
            None => {
                println!("Entity {} cannot find a path so it waits.", *e);
                ecm.set_turn(e, turn.spend_ap(1));
                ecm.remove_destination(e);
            }
        }
    }
}
