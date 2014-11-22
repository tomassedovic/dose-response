use std::collections::RingBuf;
use std::time::Duration;
use std::io;
use std::io::File;
use std::io::fs::PathExtensions;
use std::io::util::NullWriter;
use std::os;
use std::rand;
use std::rand::{IsaacRng, SeedableRng};
use std::str;

use time;

use level::Level;
use monster::Monster;
use player::Player;
use point::Point;
use systems::input::Command;
use world;
use world_gen;


#[deriving(PartialEq, Clone, Show)]
pub enum Side {
    Player,
    Computer,
}


pub struct GameState {
    pub player: Player,
    pub monsters: Vec<Monster>,
    pub level: Level,
    pub display_size: (int, int),
    pub rng: IsaacRng,
    pub commands: RingBuf<Command>,
    pub command_logger: CommandLogger,
    pub side: Side,
    pub turn: int,
    pub cheating: bool,
    pub replay: bool,
    pub clock: Duration,
    pub paused: bool,
}

impl GameState {
    fn new(width: int, height: int,
           commands: RingBuf<Command>,
           log_writer: Box<Writer+'static>,
           seed: u32,
           cheating: bool,
           replay: bool) -> GameState {
        let seed_arr: &[_] = &[seed];
        GameState {
            player: Player::new((40, 25)),
            monsters: vec![],
            level: Level::new(width, height - 2),
            display_size: (width, height),
            rng: SeedableRng::from_seed(seed_arr),
            commands: commands,
            command_logger: CommandLogger{writer: log_writer},
            side: Side::Player,
            turn: 0,
            cheating: cheating,
            replay: replay,
            clock: Duration::zero(),
            paused: false,
        }
    }

    pub fn new_game(width: int, height: int) -> GameState {
        let commands = RingBuf::new();
        let seed = rand::random::<u32>();
        let cur_time = time::now();
        let timestamp = format!("{}.{:03}",
                                time::strftime("%FT%T", &cur_time).unwrap(),
                                (cur_time.tm_nsec / 1000000));
        let replay_dir = &Path::new("./replays/");
        if !replay_dir.exists() {
            io::fs::mkdir_recursive(replay_dir,
                                    io::FilePermission::from_bits(0b111101101).unwrap()).unwrap();
        }
        let replay_path = &replay_dir.join(format!("replay-{}", timestamp));
        let mut writer = match File::create(replay_path) {
            Ok(f) => box f,
            Err(msg) => panic!("Failed to create the replay file. {}", msg)
        };
        println!("Recording the gameplay to '{}'", replay_path.display());
        writer.write_line(seed.to_string().as_slice()).unwrap();
        let mut state = GameState::new(width, height, commands, writer, seed, false, false);
        initialise_world(&mut state);
        state
    }

    pub fn replay_game(width: int, height: int) -> GameState {
        let mut commands = RingBuf::new();
        let replay_path = &Path::new(os::args()[1].as_slice());
        let mut seed: u32;
        match File::open(replay_path) {
            Ok(mut file) => {
                let bin_data = file.read_to_end().unwrap();
                let contents = str::from_utf8(bin_data.slice(0, bin_data.len()));
                let mut lines = contents.unwrap().lines();
                match lines.next() {
                    Some(seed_str) => match from_str(seed_str) {
                        Some(parsed_seed) => seed = parsed_seed,
                        None => panic!("The seed must be a number.")
                    },
                    None => panic!("The replay file is empty."),
                }
                for line in lines {
                    match from_str(line) {
                        Some(command) => commands.push_back(command),
                        None => panic!("Unknown command: {}", line),
                    }
                }
            },
            Err(msg) => panic!("Failed to read the replay file: {}. Reason: {}",
                               replay_path.display(), msg)
        }
        println!("Replaying game log: '{}'", replay_path.display());
        let mut state = GameState::new(width, height, commands, box NullWriter, seed, true, true);
        initialise_world(&mut state);
        state
    }
}

fn initialise_world(game_state: &mut GameState) {
    let (width, height) = game_state.level.size();
    // let player_pos = game_state.player.coordinates();
    world::populate_world((width, height),
                          &mut game_state.level,
                          &mut game_state.monsters,
                          game_state.player.coordinates(),
                          &mut game_state.rng,
                          world_gen::forrest);
}


pub struct CommandLogger {
    writer: Box<Writer+'static>,
}

impl CommandLogger {
    pub fn log(&mut self, command: Command) {
        self.writer.write_line(command.to_string().as_slice()).unwrap();
    }
}
