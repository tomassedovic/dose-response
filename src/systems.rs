use components::*;
use engine::{Display, Color};
use extra::container::Deque;
use extra::ringbuf::RingBuf;
use tcod::TCOD_map_t;
use tcod;
use std::rand::Rng;
use super::CommandLogger;
use path_finding::{PathFinder};
use entity_manager::EntityManager;


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

pub fn turn_system(entity: &mut GameObject, current_side: Side) {
    entity.turn.mutate(|t| if t.side == current_side {
            Turn{spent_this_turn: 0, .. t}
        } else {
            t
        });
}

pub fn input_system(entity: &mut GameObject, commands: &mut RingBuf<Command>,
                    logger: CommandLogger, current_side: Side) {
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

pub fn ai_system<T: Rng>(entity: &mut GameObject, rng: &mut T, _map: TCOD_map_t, current_side: Side) {
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

pub fn path_system(entity: &mut GameObject, map: TCOD_map_t) {
    if entity.position.is_none() { return }

    match entity.destination {
        Some(dest) => {
            let pos = entity.position.get_ref();
            entity.path = PathFinder::new(map, pos.x, pos.y, dest.x, dest.y)
                .map_move(|p| Path(p));
        },
        None => (),
    }
    entity.destination = None;
}

pub fn movement_system(entity: &mut GameObject, map: TCOD_map_t) {
    if entity.position.is_none() { return }
    if entity.path.is_none() { return }
    if entity.turn.is_none() { return }

    if entity.turn.get_ref().ap <= 0 { return }

    match (*entity.path.get_mut_ref()).walk() {
        Some((x, y)) => {
            if tcod::map_is_walkable(map, x, y) {  // Bump into the blocked entity
                entity.spend_ap(1);
                let old_pos = *entity.position.get_ref();
                entity.position = Some(Position{x: x, y: y});
                // The original position is walkable again
                tcod::map_set_properties(map, old_pos.x as uint, old_pos.y as uint, true, true);
                // Set new position walkability
                let solid = entity.solid.is_some();
                tcod::map_set_properties(map, x as uint, y as uint, true, !solid);
            } else {  // Move to the cell
                // TODO
                // entity.bump = Some(Bump(x, y));
            }
        },
        None => return,
    }
}

pub fn tile_system(entity: &GameObject, display: &mut Display) {
    if entity.position.is_none() { return }
    if entity.tile.is_none() { return }

    let &Position{x, y} = entity.position.get_ref();
    let &Tile{level, glyph, color} = entity.tile.get_ref();
    display.draw_char(level, x as uint, y as uint, glyph, color, Color(20, 20, 20));
}

pub fn idle_ai_system(entity: &mut GameObject, current_side: Side) {
    if entity.turn.is_none() { return }
    if entity.ai.is_none() { return }
    if current_side != Computer { return }

    let turn = *entity.turn.get_ref();
    let is_idle = (turn.side == current_side) && turn.spent_this_turn == 0;
    if is_idle && turn.ap > 0 { entity.spend_ap(1) };
}


pub fn end_of_turn_system(entities: &mut EntityManager<GameObject>, current_side: &mut Side) {
    let is_end_of_turn = entities.iter().all(|(_id, e)| {
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
        for (_id, e) in entities.mut_iter() {
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
