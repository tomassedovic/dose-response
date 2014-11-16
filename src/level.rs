use engine::Display;
use point::Point;
use world::col as color;


trait ToGlyph {
    fn to_glyph(&self) -> char;
}


#[deriving(PartialEq, Clone, Show)]
pub struct Cell {
    tile: Tile,
}


#[deriving(PartialEq, Clone, Rand, Show)]
pub enum Tile {
    Empty,
    Tree,
}


impl ToGlyph for Tile {
    fn to_glyph(&self) -> char {
        match *self {
            Empty => '.',
            Tree => '#',
        }
    }
}


pub struct Player {
    pos: (int, int),
}

impl Point for Player {
    fn coordinates(&self) -> (int, int) { self.pos }
}


impl ToGlyph for Player {
    fn to_glyph(&self) -> char {
        '@'
    }
}


pub struct Level {
    width: int,
    height: int,
    player: Player,
    map: Vec<Cell>,
}

impl Level {
    pub fn new(width: int, height: int) -> Level {
        assert!(width > 0 && height > 0);
        Level {
            width: width,
            height: height,
            player: Player{pos: (40, 25)},
            map: Vec::from_elem((width * height) as uint, Cell{tile: Empty}),
        }
    }

    pub fn set_tile<P: Point>(&mut self, pos: P, tile: Tile) {
        let (x, y) = pos.coordinates();
        self.map[(y * self.width + x) as uint].tile = tile;
    }

    pub fn size(&self) -> (int, int) {
        (self.width, self.height)
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn move_player<P: Point>(&mut self, new_pos: P) {
        self.player.pos = new_pos.coordinates()
    }

    pub fn render(&self, display: &mut Display) {
        let (mut x, mut y) = (0, 0);
        for cell in self.map.iter() {
            display.draw_char(0, x, y, cell.tile.to_glyph(), color::tree_1, color::background);
            x += 1;
            if x >= self.width {
                x = 0;
                y += 1;
            }
        }
        let (x, y) = self.player.pos;
        display.draw_char(2, x, y, self.player.to_glyph(), color::player, color::background);
    }
}
