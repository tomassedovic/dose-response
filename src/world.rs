use crate::{
    blocker::Blocker,
    formula,
    generators::{self, GeneratedWorld},
    item::Item,
    level::{self, Cell, Level},
    monster::Monster,
    player::PlayerInfo,
    point::{CircularArea, Point, SquareArea},
    random::Random,
    ranged_int::InclusiveRange,
    rect::Rectangle,
    state::Challenge,
};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonsterId {
    chunk_position: ChunkPosition,
    monster_index: usize,
}

/// What's the most significant thing placed on the tile?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileContents {
    Monster,
    Item,
    Irresistible,
    Empty,
}

#[derive(PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    position: Point,
    pub rng: Random,
    pub level: Level,
    monsters: Vec<Monster>,
}

impl Chunk {
    fn new(
        world_seed: u32,
        position: ChunkPosition,
        size: i32,
        player_position: Point,
        challenge: Challenge,
    ) -> Self {
        use std::num::Wrapping;
        let pos = position.position;
        // NOTE: `x` and `y` overflow on negative values here, but all
        // we care about is having a distinct value for each position
        // so our seeds don't repeat. So this is fine here.
        let seed = Wrapping(world_seed)
            + (Wrapping(13) * Wrapping(pos.x as u32))
            + (Wrapping(17) * Wrapping(pos.y as u32));

        // TODO: Monsters in different chunks will now have identical
        // IDs. We need to investigate whether that's a problem.

        let mut chunk = Chunk {
            position: pos,
            rng: Random::from_seed(seed.0),
            level: Level::new(size, size),
            monsters: vec![],
        };

        let mut throwaway_rng = chunk.rng.clone();
        let generated_data = generators::forrest::generate(
            &mut chunk.rng,
            &mut throwaway_rng,
            chunk.level.size(),
            player_position,
            challenge,
        );

        chunk.populate(generated_data);

        chunk
    }

    fn populate(&mut self, generated_world: GeneratedWorld) {
        let (map, generated_monsters, items) = generated_world;
        for &(pos, tile) in &map {
            let pos = self.level.level_position(pos);
            self.level.set_tile(pos, tile);
        }
        for mut monster in generated_monsters {
            // TODO: the pos conversion would not be necessary if the
            // worldgen operated with world positions in the first
            // place.
            let pos = self.level.level_position(monster.position);
            assert!(self.level.walkable(pos, Blocker::WALL | Blocker::MONSTER));
            monster.position = self.world_position(pos);
            self.add_monster(monster);
            assert!(!self.level.walkable(pos, Blocker::WALL | Blocker::MONSTER));
        }
        for &(pos, item) in &items {
            let pos = self.level.level_position(pos);
            assert!(self.level.walkable(pos, Blocker::WALL));
            self.level.add_item(pos, item);
        }
    }

    pub fn level_position(&self, world_position: Point) -> level::LevelPosition {
        self.level.level_position(world_position - self.position)
    }

    pub fn world_position(&self, level_position: level::LevelPosition) -> Point {
        let level_position: Point = level_position.into();
        self.position + level_position
    }

    pub fn cells(&self) -> ChunkCells<'_> {
        ChunkCells {
            chunk_position: self.position,
            cells: self.level.iter(),
        }
    }

    pub fn add_monster(&mut self, monster: Monster) -> MonsterId {
        let monster_level_position = self.level_position(monster.position);
        let monster_index = self.monsters.len();
        self.monsters.push(monster);
        self.level
            .set_monster(monster_level_position, monster_index);
        MonsterId {
            chunk_position: ChunkPosition {
                position: self.position,
            },
            monster_index,
        }
    }

    pub fn monsters(&self) -> impl Iterator<Item = &Monster> {
        self.monsters.iter().filter(|m| !m.dead)
    }

    pub fn monsters_mut(&mut self) -> impl Iterator<Item = &mut Monster> {
        self.monsters.iter_mut().filter(|m| !m.dead)
    }
}

pub struct ChunkCells<'a> {
    chunk_position: Point,
    cells: level::Cells<'a>,
}

impl<'a> Iterator for ChunkCells<'a> {
    type Item = (Point, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        self.cells.next().map(|(level_pos, cell)| {
            let offset: Point = level_pos.into();
            (self.chunk_position + offset, cell)
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ChunkPosition {
    position: Point,
}

#[derive(PartialEq, Serialize, Deserialize)]
pub struct World {
    seed: u32,
    max_half_size: i32,
    chunk_size: i32,
    chunks: HashMap<ChunkPosition, Chunk>,
    challenge: Challenge,
}

impl World {
    pub fn new(
        seed: u32,
        dimension: i32,
        chunk_size: i32,
        player_info: PlayerInfo,
        challenge: Challenge,
    ) -> World {
        log::info!("World::new(seed: {seed}, dimension: {dimension}, chunk_size: {chunk_size}, player_info: {:?}, challenge: {:?})", player_info, challenge);
        assert!(dimension > 0);
        assert!(chunk_size > 0);
        assert_eq!(dimension % 2, 0);
        assert_eq!(dimension % chunk_size, 0);

        let mut world = World {
            seed,
            max_half_size: dimension / 2,
            chunk_size,
            chunks: HashMap::new(),
            challenge,
        };

        // TODO: I don't think this code belongs in World. Move it
        // into the level generators or something?
        let mut rng = Random::from_seed(seed);
        world.prepare_initial_playing_area(player_info, &mut rng);

        world
    }

    /// Remove some of the monsters from player's initial vicinity,
    /// place some food nearby and a dose in sight.
    fn prepare_initial_playing_area(&mut self, player_info: PlayerInfo, rng: &mut Random) {
        if formula::INITIAL_SAFE_RADIUS <= formula::INITIAL_EASY_RADIUS {
            let safe_area = Rectangle::center(
                player_info.pos,
                Point::from_i32(formula::INITIAL_SAFE_RADIUS),
            );

            let easy_area = Rectangle::center(
                player_info.pos,
                Point::from_i32(formula::INITIAL_EASY_RADIUS),
            );

            for pos in easy_area.points() {
                self.ensure_chunk_at_pos(pos);
            }

            // Remove monsters from the starting area
            for pos in easy_area.points() {
                let remove_monster = self.monster_on_pos(pos).is_some_and(|m| {
                    use crate::monster::Kind::*;
                    let easy_monster = match m.kind {
                        Shadows | Voices => false,
                        Hunger | Anxiety | Depression | Npc | Signpost => true, // NOTE: Signpost should never be generated on its own
                    };
                    safe_area.contains(pos) || easy_monster
                });
                if remove_monster {
                    self.remove_monster(pos)
                }
            }

            // Remove strong doses from the starting area
            let no_lethal_dose_area = Rectangle::center(
                player_info.pos,
                Point::from_i32(formula::NO_LETHAL_DOSE_RADIUS),
            );

            // Clear any doses who's irresistible area touches the player's
            // position.
            {
                let resist_radius = formula::player_resist_radius(
                    formula::DOSE_PREFAB.irresistible,
                    player_info.will,
                );
                let resist_area =
                    Rectangle::center(player_info.pos, Point::from_i32(resist_radius));
                for point in resist_area.points() {
                    if let Some(cell) = self.cell_mut(point) {
                        for index in (0..cell.items.len()).rev() {
                            if cell.items[index].is_dose() {
                                cell.items.remove(index);
                            }
                        }
                    }
                }
            }

            for pos in no_lethal_dose_area.points() {
                if let Some(cell) = self.cell_mut(pos) {
                    for index in (0..cell.items.len()).rev() {
                        use crate::item::Kind::*;
                        let lethal_dose = match cell.items[index].kind {
                            Food | Dose => false,
                            StrongDose | CardinalDose | DiagonalDose => true,
                        };
                        if lethal_dose {
                            cell.items.remove(index);
                        }
                    }
                }
            }

            // Generate a usable dose nearby, give up after some time
            let attempts = if cfg!(feature = "recording") { 1 } else { 100 };
            for _ in 0..attempts {
                let offset = Point {
                    x: rng.range_inclusive(-3, 3),
                    y: rng.range_inclusive(-3, 3),
                };
                if offset == (0, 0) {
                    continue;
                }
                let pos = player_info.pos + offset;
                if self.walkable(pos, Blocker::WALL, player_info.pos) {
                    // Skip if there already is an item at the position
                    if !self.cell(pos).is_none_or(|cell| cell.items.is_empty()) {
                        continue;
                    }

                    let dose = formula::DOSE_PREFAB;

                    let resist_radius =
                        formula::player_resist_radius(dose.irresistible, player_info.will);
                    let resist_area = Rectangle::center(pos, Point::from_i32(resist_radius));

                    // Bail if the player would be in the resist radius
                    if resist_area.contains(player_info.pos) {
                        continue;
                    }

                    // Bail if another dose is in the resist area
                    let irresistible_dose = resist_area.points().any(|irresistible_point| {
                        self.cell(irresistible_point)
                            .is_some_and(|cell| cell.items.iter().any(Item::is_dose))
                    });
                    if irresistible_dose {
                        continue;
                    }

                    // Try to place the dose and exit
                    if let Some(chunk) = self.chunk_mut(pos) {
                        let level_position = chunk.level_position(pos);
                        chunk.level.add_item(level_position, dose);
                        break;
                    }
                }
            }

            // Generate food near the starting area, bail after 50 attempts
            let mut amount_of_food_to_generate = rng.range_inclusive(1, 3);
            for _ in 0..50 {
                let offset = Point {
                    x: rng.range_inclusive(-5, 5),
                    y: rng.range_inclusive(-5, 5),
                };
                let pos = player_info.pos + offset;
                if self.walkable(pos, Blocker::WALL, player_info.pos) {
                    let food = formula::FOOD_PREFAB;
                    if let Some(chunk) = self.chunk_mut(pos) {
                        let level_position = chunk.level_position(pos);
                        if chunk.level.cell(level_position).items.is_empty() {
                            chunk.level.add_item(level_position, food);
                            amount_of_food_to_generate -= 1;
                        }
                    }

                    if amount_of_food_to_generate <= 0 {
                        break;
                    }
                }
            }

            // Remove anything at the player's position
            if let Some(cell) = self.cell_mut(player_info.pos) {
                cell.items.clear();
            }
        }
    }
    /// Return the `ChunkPosition` for a given point within the chunk.
    ///
    /// Chunks have equal width and height and can have negative
    /// positions. There is a chunk at `(0, 0)` and then at
    /// `(-chunk_size, 0)`, `(chunk_size, 0)` and so on.
    fn chunk_pos_from_world_pos(&self, pos: Point) -> ChunkPosition {
        let chunk_pos = |num: i32| {
            let size = self.chunk_size;
            if num >= 0 {
                (num / size) * size
            } else {
                (-(((-num - 1) / size) + 1)) * size
            }
        };

        ChunkPosition {
            position: Point {
                x: chunk_pos(pos.x),
                y: chunk_pos(pos.y),
            },
        }
    }

    /// Get the chunk at the given world position. This means it
    /// doesn't have to match chunk's internal position -- any point
    /// within that Chunk will do.
    pub fn chunk(&self, pos: Point) -> Option<&Chunk> {
        let chunk_position = self.chunk_pos_from_world_pos(pos);
        self.chunks.get(&chunk_position)
    }

    /// Get the mutable chunk at the given world position. This means
    /// it doesn't have to match chunk's internal position -- any
    /// point within that Chunk will do.
    pub fn chunk_mut(&mut self, pos: Point) -> Option<&mut Chunk> {
        let chunk_position = self.chunk_pos_from_world_pos(pos);
        self.chunks.get_mut(&chunk_position)
    }

    pub fn ensure_chunk_at_pos(&mut self, pos: Point) {
        let chunk_position = self.chunk_pos_from_world_pos(pos);

        let seed = self.seed;
        let chunk_size = self.chunk_size;
        let challenge = self.challenge;
        // TODO: figure out how to generate the starting chunks so the
        // player has some doses and food and no monsters.
        self.chunks.entry(chunk_position).or_insert_with(|| {
            Chunk::new(seed, chunk_position, chunk_size, (0, 0).into(), challenge)
        });
    }

    pub fn cell(&self, world_pos: Point) -> Option<&Cell> {
        let chunk = self.chunk(world_pos);
        // NOTE: the positions within a chunk/level start from zero so
        // we need to de-offset them with the chunk position.
        chunk.map(|chunk| {
            let level_position = chunk.level_position(world_pos);
            chunk.level.cell(level_position)
        })
    }

    pub fn cell_mut(&mut self, world_pos: Point) -> Option<&mut Cell> {
        let chunk = self.chunk_mut(world_pos);
        // NOTE: the positions within a chunk/level start from zero so
        // we need to de-offset them with the chunk position.
        chunk.map(|chunk| {
            let level_position = chunk.level_position(world_pos);
            chunk.level.cell_mut(level_position)
        })
    }

    /// Check whether the given position is within the bounds of the World.
    ///
    /// While the world should be "technically infinite", we well have
    /// some sort of upper limit on the positions it's able to
    /// support.
    pub fn within_bounds(&self, pos: Point) -> bool {
        pos.x < self.max_half_size
            && pos.x > -self.max_half_size
            && pos.y < self.max_half_size
            && pos.y > -self.max_half_size
    }

    /// Check whether the given position is walkable.
    ///
    /// Points outside of the World are not walkable. The
    /// `blockers` option controls can influence the logic: are
    /// monster treated as blocking or not?
    pub fn walkable(&self, pos: Point, blockers: Blocker, player_pos: Point) -> bool {
        let level_cell_walkable = self.chunk(pos).is_some_and(|chunk| {
            let blocks_player = blockers.contains(Blocker::PLAYER) && pos == player_pos;
            let level_position = chunk.level_position(pos);
            chunk
                .level
                .walkable(level_position, blockers - Blocker::PLAYER)
                && !blocks_player
        });
        self.within_bounds(pos) && level_cell_walkable
    }

    /// Pick up the top `Item` stacked on the tile. If the position is
    /// not withing bounds, nothing happens.
    pub fn pickup_item(&mut self, pos: Point) -> Option<Item> {
        if self.within_bounds(pos) {
            self.cell_mut(pos).and_then(|cell| cell.items.pop())
        } else {
            None
        }
    }

    /// If there's a monster at the given tile, return its mutable reference.
    ///
    /// Returns `None` if there is no monster or if `pos` is out of bounds.
    pub fn monster_on_pos(&mut self, world_pos: Point) -> Option<&mut Monster> {
        if self.within_bounds(world_pos) {
            if let Some(chunk) = self.chunk_mut(world_pos) {
                let level_position = chunk.level_position(world_pos);
                chunk
                    .level
                    .monster_on_pos(level_position)
                    .map(move |monster_index| &mut chunk.monsters[monster_index])
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Return the main "thing" that's on the tile. Generally either an
    /// item, a monster or nothing.
    pub fn tile_contents(
        &self,
        world_pos: Point,
        player_will: i32,
        check_irresistible: bool,
    ) -> TileContents {
        if self.within_bounds(world_pos) {
            // NOTE: checking for the irresistible doses is quite
            // expensive (and not immediately trivial to optimise). So
            // we only do it if the caller is interested in
            // knowing about irresistible doses.
            let irresistible = if check_irresistible {
                // TODO: oof, I think this calculation is quite convoluted
                // and it's slowed down the game update times
                // significantly. Need to measure the release mode but if
                // this is a problem, we'll need to speed it up. Maybe set
                // the "irresistible" marker on the cells and just update
                // them every frame?
                self.nearest_dose(world_pos, 5)
                    .is_some_and(|(dose_pos, dose)| {
                        world_pos.tile_distance(dose_pos)
                            < formula::player_resist_radius(dose.irresistible, player_will)
                    })
            } else {
                false
            };
            if let Some(chunk) = self.chunk(world_pos) {
                let level_position = chunk.level_position(world_pos);
                let has_monster = chunk.level.monster_on_pos(level_position).is_some();
                let has_item = !chunk.level.cell(level_position).items.is_empty();
                match (has_monster, irresistible, has_item) {
                    (true, _, _) => TileContents::Monster,
                    (false, true, _) => TileContents::Irresistible,
                    (false, _, true) => TileContents::Item,
                    (false, _, false) => TileContents::Empty,
                }
            } else {
                TileContents::Empty
            }
        } else {
            TileContents::Empty
        }
    }

    /// Return a reference to a `Monster` given its `MonsterId`.
    pub fn monster(&self, id: MonsterId) -> Option<&Monster> {
        self.chunk(id.chunk_position.position)
            .and_then(|chunk| chunk.monsters.get(id.monster_index))
    }

    /// Return a mutable reference to a `Monster` given its `MonsterId`.
    pub fn monster_mut(&mut self, id: MonsterId) -> Option<&mut Monster> {
        self.chunk_mut(id.chunk_position.position)
            .and_then(|chunk| chunk.monsters.get_mut(id.monster_index))
    }

    /// Move the monster from one place in the world to the destination.
    /// If the paths are identical, nothing happens.
    /// Panics if the destination is out of bounds or already occupied.
    pub fn move_monster(
        &mut self,
        monster_position: Point,
        destination: Point,
        player_position: Point,
    ) {
        if monster_position == destination {
            return;
        }
        let blocker = Blocker::PLAYER | Blocker::MONSTER | Blocker::WALL;
        if !self.walkable(destination, blocker, player_position) {
            return;
        }

        let monster_chunk_pos = self.chunk_pos_from_world_pos(monster_position);
        let destination_chunk_pos = self.chunk_pos_from_world_pos(destination);
        if monster_chunk_pos == destination_chunk_pos {
            if let Some(monster) = self.monster_on_pos(monster_position) {
                monster.position = destination;
            }
            if let Some(chunk) = self.chunk_mut(monster_position) {
                let level_monster_pos = chunk.level_position(monster_position);
                let level_destination_pos = chunk.level_position(destination);
                chunk
                    .level
                    .move_monster(level_monster_pos, level_destination_pos);
            } else {
                log::error!("Chunk with monster {:?} doesn't exist.", monster_position)
            }
        } else {
            // Need to move the monster to another chunk
            // NOTE: We're not removing the monster from the
            // `chunk.monsters` vec in order not to mess up with the
            // indices there.
            //
            // Instead, we make it dead here (without any of the
            // normal connotations) and just remove it from the level.
            if let Some(monster) = self.monster_on_pos(monster_position) {
                let mut new_monster = monster.clone();
                monster.dead = true;

                self.remove_monster(monster_position);
                assert!(self.walkable(monster_position, blocker, player_position));
                new_monster.position = destination;
                if let Some(destination_chunk) = self.chunk_mut(destination) {
                    destination_chunk.add_monster(new_monster);
                } else {
                    log::error!("Destination chunk at {:?} doesn't exist.", destination);
                }
            } else {
                log::error!(
                    "Trying to move a monster at {}, but there's nothing there.",
                    monster_position,
                );
            }

            assert!(!self.walkable(destination, Blocker::MONSTER, player_position));
        }
    }

    /// Remove the monster at the given position (if there is any
    /// there) from the world.
    pub fn remove_monster(&mut self, pos: Point) {
        if let Some(chunk) = self.chunk_mut(pos) {
            let level_position = chunk.level_position(pos);
            let index = chunk.level.monsters.remove(&level_position);
            // TODO: we should figure out a better way of removing
            // monsters from the map.
            if let Some(index) = index {
                chunk.monsters[index].dead = true;
            }
        }
    }

    pub fn remove_monster_by_id(&mut self, id: MonsterId) {
        // TODO: this should probably be the primary way of removing
        // monsters rather than calling `remove_monster` with the
        // position here.
        let monster_pos = self.monster(id).map(|monster| monster.position);
        if let Some(monster_pos) = monster_pos {
            self.remove_monster(monster_pos);
        }
    }

    /// Set cells within the given radius as explored.
    pub fn explore(&mut self, centre: Point, radius: i32) {
        for pos in CircularArea::new(centre, radius) {
            if self.within_bounds(pos) {
                if let Some(cell) = self.cell_mut(pos) {
                    cell.explored = true;
                }
            }
        }
    }

    /// Set cells within the given radius as `always_visible`.
    pub fn always_visible(&mut self, centre: Point, radius: i32) {
        for pos in CircularArea::new(centre, radius) {
            if self.within_bounds(pos) {
                if let Some(cell) = self.cell_mut(pos) {
                    cell.explored = true;
                    cell.always_visible = true;
                }
            }
        }
    }

    /// Get a dose within the given radius that's nearest to the
    /// specified point.
    pub fn nearest_dose(&self, centre: Point, radius: i32) -> Option<(Point, Item)> {
        let mut doses = vec![];
        for pos in CircularArea::new(centre, radius) {
            // Make sure we don't go out of bounds with self.cell(pos):
            // NOTE: We're not checking for the player's position here so we'll just supply a
            // dummy value.
            if !self.walkable(pos, Blocker::WALL, Point::new(0, 0)) {
                continue;
            }
            doses.extend(
                self.cell(pos)
                    .map_or([].iter(), |cell| cell.items.iter())
                    .filter(|i| i.is_dose())
                    .map(|&item| (pos, item)),
            );
        }

        doses.pop().map(|dose| {
            let mut result = dose;
            for d in &doses {
                if centre.tile_distance(d.0) < centre.tile_distance(result.0) {
                    result = *d;
                }
            }
            result
        })
    }

    /// Return a random walkable position next to the given point.
    ///
    /// If there is no such position available, return `starting_pos`.
    pub fn random_neighbour_position(
        &self,
        rng: &mut Random,
        starting_pos: Point,
        blockers: Blocker,
        player_position: Point,
    ) -> Point {
        let mut walkables = vec![];
        for pos in SquareArea::new(starting_pos, 2) {
            if pos != starting_pos && self.walkable(pos, blockers, player_position) {
                walkables.push(pos)
            }
        }
        *rng.choose_with_fallback(&walkables, &starting_pos)
    }

    pub fn random_position_in_range(
        &self,
        rng: &mut Random,
        starting_position: Point,
        range: InclusiveRange,
        max_tries: u32,
        blockers: Blocker,
        player_position: Point,
    ) -> Option<Point> {
        for _ in 0..max_tries {
            let offset = Point::new(
                rng.range_inclusive(-range.1, range.1),
                rng.range_inclusive(-range.1, range.1),
            );
            let candidate = starting_position + offset;
            if offset.x.abs() > range.0
                && offset.y.abs() > range.0
                && self.walkable(candidate, blockers, player_position)
            {
                return Some(candidate);
            }
        }
        None
    }

    /// Returns an iterator over chunks that intersect with the given
    /// area.
    ///
    /// NOTE: The order of the chunks is not specified.
    pub fn chunks(&self, area: Rectangle) -> impl Iterator<Item = &Chunk> {
        let chunk_size = self.chunk_size;
        self.chunks
            .iter()
            .filter(move |&(pos, _chunk)| {
                let chunk_area =
                    Rectangle::from_point_and_size(pos.position, Point::from_i32(chunk_size));
                area.intersects(chunk_area)
            })
            .map(move |(_pos, chunk)| chunk)
    }

    /// Returns a mutable iterator over chunks that intersect with the
    /// given area.
    ///
    /// NOTE: The order of the chunks is not specified.
    pub fn chunks_mut(&mut self, area: Rectangle) -> impl Iterator<Item = &mut Chunk> {
        let chunk_size = self.chunk_size;
        self.chunks
            .iter_mut()
            .filter(move |&(pos, ref _chunk)| {
                let chunk_area =
                    Rectangle::from_point_and_size(pos.position, Point::from_i32(chunk_size));
                area.intersects(chunk_area)
            })
            .map(move |(_pos, chunk)| chunk)
    }

    /// Return an iterator over all monsters in the given area.
    ///
    /// NOTE: The order of the monsters is not specified.
    pub fn monsters(&self, area: Rectangle) -> impl Iterator<Item = &Monster> {
        self.chunks(area)
            .flat_map(Chunk::monsters)
            .filter(move |m| m.alive() && area.contains(m.position))
    }

    /// Return a mutable iterator over all monsters in the given area.
    ///
    /// NOTE: The order of the monsters is not specified.
    pub fn monsters_mut(&mut self, area: Rectangle) -> impl Iterator<Item = &mut Monster> {
        self.chunks_mut(area)
            .flat_map(Chunk::monsters_mut)
            .filter(move |m| m.alive() && area.contains(m.position))
    }

    pub fn positions_of_all_chunks(&self) -> Vec<Point> {
        self.chunks
            .keys()
            .map(|chunk_pos| chunk_pos.position)
            .collect()
    }
}
