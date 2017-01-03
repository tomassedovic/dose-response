use std::collections::HashMap;
use time::Duration;

use rand::{self, Rng};

use color::{self, Color};
use graphics::{self, Animation, Render};
use item::Item;
use monster::Monster;
use point::Point;


#[derive(Debug)]
pub struct Cell {
    pub tile: Tile,
    pub items: Vec<Item>,
    pub explored: bool,
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TileKind {
    Empty,
    Tree,
}

#[derive(Copy, Clone, Debug)]
pub struct Tile {
    pub kind: TileKind,
    fg_color: Color,
    animation: Animation,
    animation_state: (Duration, Color, FadeDirection),
}

#[derive(Copy, Clone, Debug)]
enum FadeDirection {
    Forward,
    Backward,
}

impl Tile {
    pub fn new(kind: TileKind) -> Tile {
        let color = match kind {
            TileKind::Empty => color::empty_tile,
            TileKind::Tree => {
                let options = [color::tree_1, color::tree_2, color::tree_3];
                *rand::thread_rng().choose(&options).unwrap()
            }
        };
        Tile {
            kind: kind,
            fg_color: color,
            animation: Animation::None,
            animation_state: (Duration::zero(), color, FadeDirection::Forward),
        }
    }

    pub fn set_animation(&mut self, animation: Animation) {
        self.animation = animation;
        match self.animation {
            Animation::None => {}
            Animation::ForegroundCycle{from, ..} => {
                self.animation_state = (Duration::zero(), from, FadeDirection::Forward);
            }
        }
    }

    pub fn update(&mut self, dt: Duration) {
        match self.animation {
            Animation::None => {}
            Animation::ForegroundCycle{from, to, duration} => {
                let (old_time, _color, old_direction) = self.animation_state;
                let mut t = old_time + dt;
                let mut direction = old_direction;

                if t > duration {
                    t = Duration::zero();
                    direction = match direction {
                        FadeDirection::Forward => FadeDirection::Backward,
                        FadeDirection::Backward => FadeDirection::Forward,
                    };
                }

                let progress = t.num_milliseconds() as f32 / duration.num_milliseconds() as f32;
                let c = match direction {
                    FadeDirection::Forward => graphics::fade_color(from, to, progress),
                    FadeDirection::Backward => graphics::fade_color(to, from, progress),
                };
                self.animation_state = (t, c, direction);
            }
        }
    }
}


impl Render for Tile {
    fn render(&self, _dt: Duration) -> (char, Color, Option<Color>) {
        use self::TileKind::*;
        use graphics::Animation::*;
        let glyph = match self.kind {
            Empty => '.',
            Tree => '#',
        };
        match self.animation {
            None => (glyph, self.fg_color, Option::None),
            ForegroundCycle{..} => {
                let (_, color, _) = self.animation_state;
                (glyph, color, Option::None)
            }
        }
    }
}


#[derive(Copy, Clone)]
pub enum Walkability {
    WalkthroughMonsters,
    BlockingMonsters,
}


pub struct Level {
    dimensions: Point,
    pub monsters: HashMap<Point, usize>,
    map: Vec<Cell>,
}

impl Level {
    pub fn new(width: i32, height: i32) -> Level {
        let dimensions = (width, height).into();
        assert!(dimensions > (0, 0));
        let map_size = (width * height) as usize;
        Level {
            dimensions: dimensions,
            monsters: HashMap::new(),
            map: (0..map_size).map(|_| Cell{
                tile: Tile::new(TileKind::Empty),
                items: vec![],
                explored: false,
            }).collect(),
        }
    }

    fn index(&self, pos: Point) -> usize {
        assert!(pos >= (0, 0));
        assert!(pos < self.dimensions);
        (pos.y * self.dimensions.x + pos.x) as usize
    }

    pub fn cell<P: Into<Point>>(&self, pos: P) -> &Cell {
        let index = self.index(pos.into());
        &self.map[index]
    }

    pub fn cell_mut<P: Into<Point>>(&mut self, pos: P) -> &mut Cell {
        let index = self.index(pos.into());
        &mut self.map[index]
    }

    pub fn set_tile<P: Into<Point>>(&mut self, pos: P, tile: Tile) {
        self.cell_mut(pos).tile = tile;
    }

    pub fn set_monster<P: Into<Point>>(&mut self, pos: P, monster_index: usize, monster: &Monster) {
        let pos = pos.into();
        assert!(monster.position == pos);
        self.monsters.insert(pos, monster_index);
    }

    pub fn monster_on_pos<P: Into<Point>>(&self, pos: P) -> Option<usize> {
        self.monsters.get(&pos.into()).map(|&ix| ix)
    }

    pub fn add_item<P: Into<Point>>(&mut self, pos: P, item: Item) {
        self.cell_mut(pos).items.push(item);
    }

    pub fn size(&self) -> Point {
        self.dimensions
    }

    pub fn within_bounds<P: Into<Point>>(&self, pos: P) -> bool {
        let pos = pos.into();
        pos >= (0, 0) && pos < self.size()
    }

    pub fn walkable<P: Into<Point>>(&self, pos: P, walkability: Walkability) -> bool {
        let pos = pos.into();
        let walkable = match walkability {
            Walkability::WalkthroughMonsters => true,
            Walkability::BlockingMonsters => self.monster_on_pos(pos).is_none(),
        };
        self.within_bounds(pos) && self.cell(pos).tile.kind == TileKind::Empty && walkable
    }

    pub fn move_monster<P: Into<Point>>(&mut self, monster_position: Point, destination: P) {
        // There can be only one monster on each cell. Bail if the destination
        // is already occupied:
        let destination = destination.into();
        assert!(!self.monsters.contains_key(&destination));
        if let Some(monster_index) = self.monsters.remove(&monster_position) {
            self.monsters.insert(destination, monster_index);
        } else {
            panic!("Moving a monster that doesn't exist");
        }
    }

    pub fn iter(&self) -> Cells {
        Cells {
            index: 0,
            width: self.dimensions.x,
            inner: self.map.iter(),
        }
    }

}

pub struct Cells<'a> {
    index: i32,
    width: i32,
    inner: ::std::slice::Iter<'a, Cell>,
}

impl<'a> Iterator for Cells<'a> {
    type Item = (Point, &'a Cell);

    fn next(&mut self) -> Option<(Point, &'a Cell)> {
        let pos = (self.index % self.width, self.index / self.width).into();
        self.index += 1;
        match self.inner.next() {
            Some(cell) => {
                Some((pos, cell))
            }
            None => None,
        }
    }
}
