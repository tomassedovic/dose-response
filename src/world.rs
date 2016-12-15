use level::{Level, Walkability};
use monster::Monster;
use generators::GeneratedWorld;

use rand::IsaacRng;

pub struct Chunk {
    pub rng: IsaacRng,
    pub level: Level,
}


pub fn populate_world(level: &mut Level,
                      monsters: &mut Vec<Monster>,
                      generated_world: GeneratedWorld) {
    let (map, generated_monsters, items) = generated_world;
    for &(pos, item) in map.iter() {
        level.set_tile(pos, item);
    }
    for &(pos, kind) in generated_monsters.iter() {
        assert!(level.walkable(pos, Walkability::BlockingMonsters));
        let monster = Monster::new(kind, pos);
        monsters.push(monster);
    }
    for &(pos, item) in items.iter() {
        assert!(level.walkable(pos, Walkability::BlockingMonsters));
        level.add_item(pos, item);
    }
}
