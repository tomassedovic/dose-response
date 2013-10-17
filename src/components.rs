use engine::{Color};
pub use map::PathResource;
pub use self::ai::AI;
use entity_manager;

#[deriving(Eq)]
pub enum Side {
    Player,
    Computer,
}

mod ai {
    pub enum Behaviour {
        Individual,
        Pack,
    }

    pub enum State {
        Idle,
        Aggressive,
    }

    pub struct AI{behaviour: Behaviour, state: State}
}

pub struct AcceptsUserInput;
pub struct Attack(entity_manager::ID);
pub struct Background;
pub struct Bump(entity_manager::ID);
pub struct Position {x: int, y: int}
pub struct Destination {x: int, y: int}
pub struct Solid;
pub struct Tile{level: uint, glyph: char, color: Color}
pub struct Turn{side: Side, ap: int, max_ap: int, spent_this_turn: int}

pub struct GameObject {
    ai: Option<AI>,
    accepts_user_input: Option<AcceptsUserInput>,
    attack: Option<Attack>,
    background: Option<Background>,
    bump: Option<Bump>,
    position: Option<Position>,
    destination: Option<Destination>,
    path: Option<PathResource>,
    solid: Option<Solid>,
    tile: Option<Tile>,
    turn: Option<Turn>,
}

impl GameObject {
    pub fn new() -> GameObject {
        GameObject {
            ai: None,
            accepts_user_input: None,
            attack: None,
            background: None,
            bump: None,
            position: None,
            destination: None,
            path: None,
            solid: None,
            tile: None,
            turn: None,
        }
    }

    pub fn spend_ap(&mut self, spend: int) {
        match self.turn {
            Some(turn) => {
                assert!(spend <= turn.ap);

                self.turn = Some(Turn{
                        ap: turn.ap - spend,
                        spent_this_turn: turn.spent_this_turn + spend,
                        .. turn});
            },
            None => fail!(),
        }
    }
}
