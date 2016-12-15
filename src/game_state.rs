use std::collections::{VecDeque, HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, BufRead, Write};
use std::path::Path;

use time;
use time::Duration;
use rand::{self, IsaacRng, SeedableRng};

use generators;
use level::Level;
use monster::Monster;
use player::Player;
use point::Point;
use world::{self, Chunk};


// TODO: Rename this to `GameState` and the existing `GameState` to
// `Game`? It's no longer just who's side it is but also: did the
// player won? Lost?
#[derive(Copy, PartialEq, Clone, Debug)]
pub enum Side {
    Player,
    Computer,
    Victory,
}


// TODO: rename this to Input or something like that. This represents the raw
// commands from the player or AI abstracted from keyboard, joystick or
// whatever. But they shouldn't carry any context or data.
#[derive(Copy, Clone, Debug)]
pub enum Command {
    N, E, S, W,
    NE, NW, SE, SW,
    UseFood,
    UseDose,
    UseStrongDose,
}

impl Command {
    fn to_str(&self) -> &'static str {
    use self::Command::*;
        match *self {
            N => "N",
            E => "E",
            S => "S",
            W => "W",
            NE => "NE",
            NW => "NW",
            SE => "SE",
            SW => "SW",
            UseFood => "UseFood",
            UseDose => "UseDose",
            UseStrongDose => "UseStrongDose",
        }
    }
}


fn command_from_str(name: &str) -> Command {
    use self::Command::*;
    match name {
        "N" => N,
        "E" => E,
        "S" => S,
        "W" => W,
        "NE" => NE,
        "NW" => NW,
        "SE" => SE,
        "SW" => SW,
        "UseFood" => UseFood,
        "UseDose" => UseDose,
        "UseStrongDose" => UseStrongDose,
        _ => panic!("Unknown command: '{}'", name)
    }
}


// TODO: remove when this exists in the stable standard library (it prolly does now)
fn path_exists(path: &Path) -> bool {
    ::std::fs::metadata(path).is_ok()
}

/// Return the world position of the chunk which contains the point
/// passed in.
fn chunk_from_world_pos<P: Into<Point>>(world_pos: P) -> Point {
    unimplemented!()
}


pub struct GameState {
    pub player: Player,
    pub monsters: Vec<Monster>,
    pub explosion_animation: super::ExplosionAnimation,

    /// The actual size of the game world in tiles. Could be infinite
    /// but we're limiting it for performance reasons for now.
    pub world_size: Point,
    pub chunk_size: i32,
    pub world: HashMap<Point, Chunk>,

    /// The size of the game map inside the game window. We're keeping
    /// this square so this value repesents both width and heigh.
    /// It's a window into the game world that is actually rendered.
    pub map_size: i32,

    /// The width of the in-game status panel.
    pub panel_width: i32,

    /// The size of the game window in tiles. The area stuff is
    /// rendered to. NOTE: currently, the width is equal to map_size +
    /// panel_width, height is map_size.
    pub display_size: Point,
    pub screen_position_in_world: Point,
    pub seed: u32,
    pub rng: IsaacRng,
    pub commands: VecDeque<Command>,
    pub command_logger: Box<Write>,
    pub side: Side,
    pub turn: i32,
    pub cheating: bool,
    pub replay: bool,
    pub clock: Duration,
    pub pos_timer: ::Timer,
    pub paused: bool,
    pub old_screen_pos: Point,
    pub new_screen_pos: Point,
    pub screen_fading: Option<super::ScreenFadeAnimation>,
    pub see_entire_screen: bool,
}

impl GameState {
    fn new<W: Write+'static>(world_size: Point,
                             map_size: i32,
                             panel_width: i32,
                             display_size: Point,
                             commands: VecDeque<Command>,
                             log_writer: W,
                             seed: u32,
                             cheating: bool,
                             replay: bool)
                             -> GameState {
        let seed_arr: &[_] = &[seed];
        let world_centre = world_size / 2;
        assert_eq!(display_size, (map_size + panel_width, map_size));
        GameState {
            player: Player::new(world_centre),
            monsters: vec![],
            explosion_animation: None,
            chunk_size: 32,
            world_size: world_size,
            world: HashMap::new(),
            map_size: map_size,
            panel_width: panel_width,
            display_size: display_size,
            screen_position_in_world: world_centre,
            seed: seed,
            rng: SeedableRng::from_seed(seed_arr),
            commands: commands,
            command_logger: Box::new(log_writer),
            side: Side::Player,
            turn: 0,
            cheating: cheating,
            replay: replay,
            clock: Duration::zero(),
            pos_timer: ::Timer::new(Duration::milliseconds(0)),
            old_screen_pos: (0, 0).into(),
            new_screen_pos: (0, 0).into(),
            paused: false,
            screen_fading: None,
            see_entire_screen: false,
        }
    }

    pub fn new_game(world_size: Point, map_size: i32, panel_width: i32, display_size: Point) -> GameState {
        let commands = VecDeque::new();
        let seed = rand::random::<u32>();
        let cur_time = time::now();
        // Timestamp in format: 2016-11-20T20-04-39.123
        // We can't use the colons in the timestamp -- Windows don't allow them in a path.
        let timestamp = format!("{}.{:03}",
                                time::strftime("%FT%H-%M-%S", &cur_time).unwrap(),
                                (cur_time.tm_nsec / 1000000));
        let replay_dir = &Path::new("replays");
        assert!(replay_dir.is_relative());
        if !path_exists(replay_dir) {
            fs::create_dir_all(replay_dir).unwrap();
        }
        let replay_path = &replay_dir.join(format!("replay-{}", timestamp));
        let mut writer = match File::create(replay_path) {
            Ok(f) => f,
            Err(msg) => panic!("Failed to create the replay file at '{:?}'.\nReason: '{}'.",
                               replay_path.display(), msg),
        };
        // println!("Recording the gameplay to '{}'", replay_path.display());
        log_seed(&mut writer, seed);
        let mut state = GameState::new(world_size, map_size, panel_width, display_size, commands, writer, seed, false, false);
        initialise_world(&mut state);
        state
    }

    pub fn replay_game(world_size: Point, map_size: i32, panel_width: i32, display_size: Point) -> GameState {
        let mut commands = VecDeque::new();
        let path_str = env::args().nth(1).unwrap();
        let replay_path = &Path::new(&path_str);
        let seed: u32;
        match File::open(replay_path) {
            Ok(file) => {
                let mut lines = BufReader::new(file).lines();
                match lines.next() {
                    Some(seed_str) => match seed_str.unwrap().parse() {
                        Ok(parsed_seed) => seed = parsed_seed,
                        Err(_) => panic!("The seed must be a number.")
                    },
                    None => panic!("The replay file is empty."),
                }
                for line in lines {
                    match line {
                        Ok(line) => commands.push_back(command_from_str(&line)),
                        Err(err) => panic!("Error reading a line from the replay file: {:?}.", err),
                    }
                }
            },
            Err(msg) => panic!("Failed to read the replay file: {}. Reason: {}",
                               replay_path.display(), msg)
        }
        // println!("Replaying game log: '{}'", replay_path.display());
        let mut state = GameState::new(world_size, map_size, panel_width, display_size, commands, Box::new(io::sink()), seed, true, true);
        initialise_world(&mut state);
        state
    }
}

fn initialise_world(state: &mut GameState) {
    assert!(state.map_size >= state.chunk_size);
    let map_dimensions: Point = (state.map_size, state.map_size).into();
    let left_top_corner = state.screen_position_in_world - map_dimensions / 2;
    // NOTE: The world goes from (0, 0) onwards. So `x / chunk_size`
    // gives you the horizontal coordinate of the chunk containing
    // your `x`.
    let min_x_chunk = left_top_corner.x / state.chunk_size;
    let x_cells_to_fill = left_top_corner.x - min_x_chunk + state.map_size;
    let x_chunks = if x_cells_to_fill % state.chunk_size == 0 {
        x_cells_to_fill / state.chunk_size
    } else {
        x_cells_to_fill / state.chunk_size + 1
    };

    let min_y_chunk = left_top_corner.y / state.chunk_size;
    let y_cells_to_fill = left_top_corner.y - min_y_chunk + state.map_size;
    let y_chunks = if y_cells_to_fill % state.chunk_size == 0 {
        y_cells_to_fill / state.chunk_size
    } else {
        y_cells_to_fill / state.chunk_size + 1
    };

    let min_chunk_pos = Point::new(min_x_chunk, min_y_chunk);

    for x_chunk_increment in 0..x_chunks {
        for y_chunk_increment in 0..y_chunks {
            let chunk_pos = min_chunk_pos + (x_chunk_increment, y_chunk_increment);
            assert!(chunk_pos.x >= 0);
            assert!(chunk_pos.y >= 0);

            let chunk_seed: &[_] = &[state.seed, chunk_pos.x as u32, chunk_pos.y as u32];
            let mut chunk = Chunk {
                rng: SeedableRng::from_seed(chunk_seed),
                level: Level::new(state.chunk_size, state.chunk_size),
            };

            let generated_level = generators::forrest::generate(&mut chunk.rng,
                                                                chunk.level.size(),
                                                                state.player.pos);
            world::populate_world(&mut chunk.level,
                                  &mut state.monsters,
                                  generated_level);

            state.world.insert(chunk_pos, chunk);
        }
    }

    // TODO: Can we keep monsters in a global list or do we have to partition them as well?
    // Sort monsters by their APs, set their IDs to equal their indexes in state.monsters:
    state.monsters.sort_by(|a, b| b.max_ap.cmp(&a.max_ap));
    for (index, m) in state.monsters.iter_mut().enumerate() {
        // TODO: UGH. Just use an indexed entity store that pops these up.
        unsafe {
            m.set_id(index);
        }
        let chunk_pos = chunk_from_world_pos(m.position);
        match state.world.entry(chunk_pos) {
            Occupied(chunk) => chunk.get_mut().level.set_monster(m.position, m.id(), m),
            Vacant(_) => unreachable!()  // All monsters should belong to a chunk
        }
    }
}


pub fn log_seed<W: Write>(writer: &mut W, seed: u32) {
    writeln!(writer, "{}", seed).unwrap();
}

pub fn log_command<W: Write>(writer: &mut W, command: Command) {
    writeln!(writer, "{}", command.to_str()).unwrap();
}
