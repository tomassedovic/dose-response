use engine::{Color};
pub use map::Path;

#[deriving(Eq)]
pub enum Side {
    Player,
    Computer,
}

pub struct AcceptsUserInput;
pub struct AI;
pub struct Background;
pub struct Position {x: int, y: int}
pub struct Destination {x: int, y: int}
pub struct Solid;
pub struct Tile{level: uint, glyph: char, color: Color}
pub struct Turn{side: Side, ap: int, max_ap: int, spent_this_turn: int}

pub struct GameObject {
    ai: Option<AI>,
    accepts_user_input: Option<AcceptsUserInput>,
    background: Option<Background>,
    position: Option<Position>,
    destination: Option<Destination>,
    path: Option<Path>,
    solid: Option<Solid>,
    tile: Option<Tile>,
    turn: Option<Turn>,
}

impl GameObject {
    pub fn new() -> GameObject {
        GameObject {
            ai: None,
            accepts_user_input: None,
            background: None,
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
