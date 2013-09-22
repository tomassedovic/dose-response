use components::*;
use engine::{Display, Color};
use extra::deque::Deque;
use tcod::TCOD_map_t;
use tcod;
use std::rand::RngUtil;
use super::CommandLogger;

#[deriving(Rand, ToStr)]
pub enum Command {
    N, E, S, W, NE, NW, SE, SW,
}

impl Command {
    pub fn from_str(name: &str) -> Option<Command> {
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

pub fn input_system(entity: &mut GameObject, commands: &mut Deque<Command>,
                    logger: CommandLogger, current_side: Side) {
    if entity.accepts_user_input.is_none() { return }
    if entity.position.is_none() { return }
    if commands.is_empty() { return }
    match current_side {
        Player => (),
        _ => return,
    }

    let pos = entity.position.get();
    let command = commands.pop_front();
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
}

pub fn ai_system<T: RngUtil>(entity: &mut GameObject, rng: &mut T, map: TCOD_map_t, current_side: Side) {
    if entity.ai.is_none() { return }
    if entity.position.is_none() { return }
    match current_side {
        Computer => (),
        _ => return,
    }

    let pos = entity.position.get();
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

pub fn movement_system(entity: &mut GameObject, map: TCOD_map_t) {
    if entity.position.is_none() { return }
    if entity.destination.is_none() { return }
    if entity.turn.is_none() { return }

    let turn = entity.turn.get();
    if turn.ap <= 0 { return }

    let old_pos = entity.position.get();
    let Destination{x, y} = entity.destination.get();
    let (width, height) = tcod::map_size(map);
    if x < 0 || y < 0 || x >= width as int || y >= height as int {
        // reached the edge of the screen
    } else if tcod::map_is_walkable(map, x as uint, y as uint) {
        entity.spend_ap(1);
        entity.position = Some(Position{x: x, y: y});
        // The original position is walkable again
        tcod::map_set_properties(map, old_pos.x as uint, old_pos.y as uint, true, true);
        // Set new position walkability
        let solid = entity.solid.is_some();
        tcod::map_set_properties(map, x as uint, y as uint, true, !solid);
    } else { /* path is blocked */ }
    entity.destination = None;
}

pub fn tile_system(entity: &GameObject, display: &mut Display) {
    if entity.position.is_none() { return }
    if entity.tile.is_none() { return }

    let Position{x, y} = entity.position.get();
    let Tile{level, glyph, color} = entity.tile.get();
    display.draw_char(level, x as uint, y as uint, glyph, color, Color(20, 20, 20));
}

pub fn idle_ai_system(entity: &mut GameObject, current_side: Side) {
    if entity.turn.is_none() { return }
    if entity.ai.is_none() { return }
    if current_side != Computer { return }

    let turn = entity.turn.get();
    let is_idle = (turn.side == current_side) && turn.spent_this_turn == 0;
    if is_idle && turn.ap > 0 { entity.spend_ap(1) };
}


pub fn end_of_turn_system(entities: &mut [GameObject], current_side: &mut Side) {
    let is_end_of_turn = entities.iter().all(|e| {
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
        for entities.mut_iter().filter(|&e| e.turn.is_some() ).advance |e| {
            let t = e.turn.get();
            if t.side == *current_side {
                e.turn = Some(Turn{side: t.side, ap: t.max_ap,
                                   spent_this_turn: 0, .. t});
            }
        }
    }
}
