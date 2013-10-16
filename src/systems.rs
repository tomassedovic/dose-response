use components::*;
use engine::{Display, Color};
use extra::container::Deque;
use extra::ringbuf::RingBuf;
use map;
use std::rand::Rng;
use super::CommandLogger;
use entity_manager::{EntityManager, ID};


#[deriving(Rand, ToStr)]
pub enum Command {
    N, E, S, W, NE, NW, SE, SW,
}

impl FromStr for Command {
    fn from_str(name: &str) -> Option<Command> {
        match name {
            "N" => Some(N),
            "E" => Some(E),
            "S" => Some(S),
            "W" => Some(W),
            "NE" => Some(NE),
            "NW" => Some(NW),
            "SE" => Some(SE),
            "SW" => Some(SW),
            _ => None,
        }
    }
}

pub fn turn_system(id: ID, ecm: &mut EntityManager<GameObject>, current_side: Side) {
    match ecm.get_mut_ref(id) {
        Some(entity) => {
            entity.turn.mutate(|t| if t.side == current_side {
                    Turn{spent_this_turn: 0, .. t}
                } else {
                    t
                });
        }
        None => {}
    }
}

pub fn input_system(id: ID, ecm: &mut EntityManager<GameObject>, commands: &mut RingBuf<Command>,
                    logger: CommandLogger, current_side: Side) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_mut_ref(id).unwrap();

    if entity.accepts_user_input.is_none() { return }
    if entity.position.is_none() { return }
    match current_side {
        Player => (),
        _ => return,
    }

    let pos = entity.position.get_ref();
    match commands.pop_front() {
        Some(command) => {
            logger.log(command);
            let dest = match command {
                N => Destination{x: pos.x, y: pos.y-1},
                S => Destination{x: pos.x, y: pos.y+1},
                W => Destination{x: pos.x-1, y: pos.y},
                E => Destination{x: pos.x+1, y: pos.y},

                NW => Destination{x: pos.x-1, y: pos.y-1},
                NE => Destination{x: pos.x+1, y: pos.y-1},
                SW => Destination{x: pos.x-1, y: pos.y+1},
                SE => Destination{x: pos.x+1, y: pos.y+1},
            };
            entity.destination = Some(dest);
        },
        None => (),
    }
}

pub fn ai_system<T: Rng>(id: ID, ecm: &mut EntityManager<GameObject>, rng: &mut T, _map: &map::Map, current_side: Side) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_mut_ref(id).unwrap();

    if entity.ai.is_none() { return }
    if entity.position.is_none() { return }
    match current_side {
        Computer => (),
        _ => return,
    }

    let pos = entity.position.get_ref();
    let dest = match rng.gen::<Command>() {
        N => Destination{x: pos.x, y: pos.y-1},
        S => Destination{x: pos.x, y: pos.y+1},
        W => Destination{x: pos.x-1, y: pos.y},
        E => Destination{x: pos.x+1, y: pos.y},

        NW => Destination{x: pos.x-1, y: pos.y-1},
        NE => Destination{x: pos.x+1, y: pos.y-1},
        SW => Destination{x: pos.x-1, y: pos.y+1},
        SE => Destination{x: pos.x+1, y: pos.y+1},
    };
    entity.destination = Some(dest);

}

pub fn path_system(id: ID, ecm: &mut EntityManager<GameObject>, map: &mut map::Map) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_mut_ref(id).unwrap();

    if entity.position.is_none() { return }

    match entity.destination {
        Some(dest) => {
            let pos = entity.position.get_ref();
            entity.path = map.find_path((pos.x, pos.y), (dest.x, dest.y));
        },
        None => (),
    }
    entity.destination = None;
}

pub fn movement_system(id: ID, ecm: &mut EntityManager<GameObject>, map: &mut map::Map) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_mut_ref(id).unwrap();

    if entity.position.is_none() { return }
    if entity.path.is_none() { return }
    if entity.turn.is_none() { return }

    if entity.turn.get_ref().ap <= 0 { return }

    match (*entity.path.get_mut_ref()).walk() {
        Some((x, y)) => {
            if map.is_walkable((x, y)) {  // Move to the cell
                entity.spend_ap(1);
                // Update both the entity position component and the map:
                match *entity.position.get_ref() {
                    Position{x: oldx, y: oldy} => {
                        map.move_entity(*id, (oldx, oldy), (x, y));
                        entity.position = Some(Position{x: x, y: y});
                    }
                };
            } else {  // Bump into the blocked entity
                // TODO: assert there's only one solid entity on pos [x, y]
                for (bumpee, walkable) in map.entities_on_pos((x, y)) {
                    assert!(bumpee != *id);
                    match walkable {
                        map::Walkable => loop,
                        map::Solid => {
                            println!("Entity {} bumped into {} at: ({}, {})", *id, bumpee, x, y);
                            entity.bump = Some(Bump(ID(bumpee)));
                            break;
                        }
                    }
                }
            }
        },
        None => return,
    }
}


pub fn bump_system(entity_id: ID, ecm: &mut EntityManager<GameObject>) {
    let bumpee_id = {match ecm.get_ref(entity_id).unwrap().bump {Some(id) => *id, None => {return}}};
    let bumpee = {ecm.get_ref(bumpee_id).unwrap().turn};
    match ecm.get_mut_ref(entity_id) {
        Some(e) => {
            if bumpee.is_some() && e.turn.is_some() && bumpee.unwrap().side != e.turn.unwrap().side {
                println!("Entity {} attacks {}.", *entity_id, *bumpee_id)
                e.attack = Some(Attack(bumpee_id));
            } else {
                println!("Entity {} hits the wall.", *entity_id);
            }
            e.bump = None;
        }
        _ => (),
    }
}

pub fn combat_system(id: ID, ecm: &mut EntityManager<GameObject>, map: &mut map::Map) {
    let free_aps = match ecm.get_ref(id) {
        Some(e) => {
            match e.turn {
                Some(t) => t.ap,
                None => 0
            }
        }
        None => 0,
    };
    let target_id = match ecm.get_ref(id) {
        Some(e) => match e.attack {
            Some(attack_component) => *attack_component,
            None => return,
        },
        None => { return }
    };
    let attack_successful = ecm.get_ref(target_id).is_some() && free_aps > 0;
    if attack_successful {
        // attacker spends an AP
        match ecm.get_mut_ref(id) {
            Some(attacker) => {
                attacker.spend_ap(1);
                attacker.attack = None;
            }
            None => {}
        }
        // kill the target
        match ecm.get_mut_ref(target_id) {
            Some(target) => {
                target.ai = None;
                match target.position {
                    Some(Position{x, y}) => {
                        target.position = None;
                        map.remove_entity(*target_id, (x, y));
                    }
                    None => {}
                }
                target.accepts_user_input = None;
                target.turn = None;
            }
            None => {}
        }
    }
}

pub fn tile_system(id: ID, ecm: &EntityManager<GameObject>, display: &mut Display) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_ref(id).unwrap();

    if entity.position.is_none() { return }
    if entity.tile.is_none() { return }

    let &Position{x, y} = entity.position.get_ref();
    let &Tile{level, glyph, color} = entity.tile.get_ref();
    display.draw_char(level, x as uint, y as uint, glyph, color, Color(20, 20, 20));
}

pub fn idle_ai_system(id: ID, ecm: &mut EntityManager<GameObject>, current_side: Side) {
    if ecm.get_ref(id).is_none() { return }
    let entity = ecm.get_mut_ref(id).unwrap();

    if entity.turn.is_none() { return }
    if entity.ai.is_none() { return }
    if current_side != Computer { return }

    let turn = *entity.turn.get_ref();
    let is_idle = (turn.side == current_side) && turn.spent_this_turn == 0;
    if is_idle && turn.ap > 0 { entity.spend_ap(1) };
}


pub fn end_of_turn_system(entities: &mut EntityManager<GameObject>, current_side: &mut Side) {
    let is_end_of_turn = entities.iter().all(|(e, _id)| {
            match e.turn {
                Some(turn) => {
                    *current_side != turn.side || turn.ap == 0
                },
                None => true,
            }
        });
    if is_end_of_turn {
        *current_side = match *current_side {
            Player => Computer,
            Computer => Player,
        };
        for (e, _id) in entities.mut_iter() {
            match e.turn {
                Some(turn) => {
                    if turn.side == *current_side {
                        e.turn = Some(Turn{side: turn.side, ap: turn.max_ap,
                                        spent_this_turn: 0, .. turn});
                    }
                },
                None => (),
            }
        }
    }
}

pub fn player_dead_system(id: ID, ecm: &mut EntityManager<GameObject>, player_id: ID) {
    let player_dead = match ecm.get_ref(player_id) {
        Some(player) => {
            player.position.is_none() || player.turn.is_none()
        }
        None => fail!("Could not find the Player entity (id: %?)", player_id),
    };
    if player_dead {
        match ecm.get_mut_ref(id) {
            Some(e) => e.ai = None,
            None => (),
        }
    }
}
