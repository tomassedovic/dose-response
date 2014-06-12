use std::rand::Rng;
use std::rand::distributions::{Weighted, WeightedChoice, IndependentSample};

#[deriving(PartialEq, Clone, Rand)]
pub enum WorldItem {
    Empty,
    Tree,

    Dose,
    StrongDose,
    Food,

    Anxiety,
    Depression,
    Hunger,
    Voices,
    Shadows,
}

pub fn forrest<T: Rng>(rng: &mut T, w: int, h: int) -> Vec<(int, int, WorldItem)> {
    let monster_count = 5;
    let monster_weight = 30 / monster_count;
    let mut weights = [
        Weighted{weight: 595, item: Empty},
        Weighted{weight: 390, item: Tree},
        Weighted{weight: 7,  item: Dose},
        Weighted{weight: 3,  item: StrongDose},
        Weighted{weight: 5,  item: Food},
        Weighted{weight: monster_weight,  item: Anxiety},
        Weighted{weight: monster_weight,  item: Depression},
        Weighted{weight: monster_weight,  item: Hunger},
        Weighted{weight: monster_weight,  item: Voices},
        Weighted{weight: monster_weight,  item: Shadows},
    ];
    let opts = WeightedChoice::new(weights);
    let mut result: Vec<(int, int, WorldItem)> = vec![];
    for x in range(0, w) {
        for y in range(0, h) {
            result.push((x, y, opts.ind_sample(rng)));
        }
    }
    result
}
