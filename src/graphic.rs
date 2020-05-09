use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Graphic {
    Empty,
    Tree1,
    Tree2,
    Tree3,
    Tree4,
    Tree5,
    Tree6,
    Tree7,
    Tree8,
    Tree9,
    Tree10,

    Ground1,
    Ground2,
    Ground3,
    Ground4,
    Ground5,

    Grass1,
    Grass2,
    Grass3,
    Grass4,
    Grass5,
    Grass6,
    Grass7,
    Grass8,
    Grass9,

    Player,
    Npc,
    Corpse,

    Anxiety,
    Depression,
    Hunger,
    Shadows,
    Voices,

    Dose,
    StrongDose,
    CardinalDose,
    DiagonalDose,
    Food,

    Signpost,
}

impl Into<char> for Graphic {
    fn into(self) -> char {
        use Graphic::*;
        match self {
            Empty => ' ',
            Tree1 => '#',
            Tree2 => '#',
            Tree3 => '#',
            Tree4 => '#',
            Tree5 => '#',
            Tree6 => '#',
            Tree7 => '#',
            Tree8 => '#',
            Tree9 => '#',
            Tree10 => '#',
            Ground1 => '.',
            Ground2 => '.',
            Ground3 => '.',
            Ground4 => '.',
            Ground5 => '.',
            Grass1 => '.',
            Grass2 => '.',
            Grass3 => '.',
            Grass4 => '.',
            Grass5 => '.',
            Grass6 => '.',
            Grass7 => '.',
            Grass8 => '.',
            Grass9 => '.',
            Player => '@',
            Npc => '@',
            Corpse => '&',
            Anxiety => 'a',
            Depression => 'D',
            Hunger => 'h',
            Shadows => 'S',
            Voices => 'v',
            Dose => 'i',
            StrongDose => 'I',
            CardinalDose => '+',
            DiagonalDose => 'x',
            Food => '%',
            Signpost => '!',
        }
    }
}
