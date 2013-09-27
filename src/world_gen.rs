use std::rand::{Rng, Weighted};

#[deriving(Clone, Rand, Eq)]
pub enum WorldItem {
    Empty,
    Tree,

    Dose,
    StrongDose,

    Anxiety,
    Depression,
    Hunger,
    Voices,
    Shadows,
}

pub fn forrest<T: Rng>(rng: &mut T, w: uint, h: uint) -> ~[(int, int, WorldItem)] {
    let monster_count = 5;
    let monster_weight = 30 / monster_count;
    let opts = [
        Weighted{weight: 600, item: Empty},
        Weighted{weight: 390, item: Tree},
        Weighted{weight: 7,  item: Dose},
        Weighted{weight: 3,  item: StrongDose},
        Weighted{weight: monster_weight,  item: Anxiety},
        Weighted{weight: monster_weight,  item: Depression},
        Weighted{weight: monster_weight,  item: Hunger},
        Weighted{weight: monster_weight,  item: Voices},
        Weighted{weight: monster_weight,  item: Shadows},
    ];
    let mut result: ~[(int, int, WorldItem)] = ~[];
    for x in range(0, w) {
        for y in range(0, h) {
            result.push((x as int, y as int,
                       rng.choose_weighted(opts)));
        }
    }
    result
}
