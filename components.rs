use engine::{Color};

pub struct AcceptsUserInput;
pub struct AI;
pub struct Position {x: int, y: int}
pub struct Destination {x: int, y: int}
pub struct Health(int);
pub struct Solid;
pub struct Tile{level: uint, glyph: char, color: Color}

pub struct GameObject {
    ai: Option<AI>,
    accepts_user_input: Option<AcceptsUserInput>,
    position: Option<Position>,
    destination: Option<Destination>,
    health: Option<Health>,
    solid: Option<Solid>,
    tile: Option<Tile>,
}

impl GameObject {
    pub fn new() -> GameObject {
        GameObject {
            ai: None,
            accepts_user_input: None,
            position: None,
            destination: None,
            health: None,
            solid: None,
            tile: None,
        }
    }
}