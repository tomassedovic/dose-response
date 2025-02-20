use crate::{
    ai,
    animation::{self, AreaOfEffect},
    audio::{Audio, Effect},
    blocker::Blocker,
    color,
    engine::{Display, Mouse, TextMetrics},
    formula,
    gamepad::Gamepad,
    graphic::Graphic,
    item,
    keys::{Key, KeyCode, Keys},
    level::TileKind,
    monster::{self, CompanionBonus},
    palette::Palette,
    pathfinding,
    player::{self, Modifier},
    point::{self, Point},
    random::Random,
    ranged_int::{InclusiveRange, Ranged},
    rect::Rectangle,
    render,
    settings::{Settings, Store as SettingsStore},
    state::{
        self, Challenge, Command, GameSession, Input, MotionAnimation, Side, State,
        VerificationWrapper,
    },
    stats::{FrameStats, Stats},
    timer::{Stopwatch, Timer},
    ui, util,
    window::{self, Window},
    windows::{endgame, help, main_menu, message, settings, sidebar},
    world::World,
};

use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use egui::{CtxRef, Ui};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Action {
    Move(Point),
    Attack(Point, player::Modifier),
    Use(item::Kind),
}

pub enum RunningState {
    Running,
    Stopped,
    /// Skip the rendering and FPS-related timing. Instead, proceed to
    /// the next update immediately.
    Skip,
    NewGame(Box<State>),
}

pub fn update(
    state: &mut State,
    egui_ctx: &CtxRef,
    dt: Duration,
    fps: i32,
    new_keys: &[Key],
    mouse: Mouse,
    gamepad: &mut Gamepad,
    settings: &mut Settings,
    metrics: &dyn TextMetrics,
    settings_store: &mut dyn SettingsStore,
    display: &mut Display, // TODO: remove this from the entgine and keep a transient state instead
    audio: &mut Audio,
) -> RunningState {
    let update_stopwatch = Stopwatch::start();
    state.clock += dt;
    state.replay_step += dt;

    // NOTE: we need to use `input.keys` because `state.keys` will
    // only have the replay keys, not any new key presses.
    //
    // We also need to set this early before anything else (such as
    // tick_id increase) happens so everything can work with the
    // pause.
    if state.replay {
        let mut keys = Keys::new();
        keys.extend(new_keys.iter().copied());
        state.paused = if keys.matches_code(KeyCode::Space) {
            !state.paused
        } else {
            state.paused
        };
    }

    if state.window_stack.top() == Window::Game && !state.paused {
        state.previous_tick = state.tick_id;
        state.tick_id += 1;
        log::debug!("Starting new tick with ID: {}", state.tick_id);
    } else {
        //log::warn!("NOT A GAME FRAME!!");
    }

    // The music won't play during the initial main menu screen so
    // start it after the game starts and then just keep playing
    // forever.
    if state.game_session.started() {
        audio.background_sound_queue.play();
    }

    // TODO: only check this every say 10 or 100 frames?
    // We just wanna make sure there are items in the queue.
    enqueue_background_music(audio, &mut state.audio_rng);

    audio.set_background_volume(settings.background_volume);
    audio.set_effects_volume(settings.sound_volume);

    // TODO: remove `state.map_size` if we're always recalculating it
    // here anyway?? The map size now depends solely on the window
    // size in pixels anyway so there's little sense carrying it
    // around.
    let panel_width_tiles = (formula::sidebar_width_px(settings.text_size) as f32
        / settings.tile_size as f32)
        .ceil() as i32;
    if display.size_without_padding() != (state.map_size.x + panel_width_tiles, state.map_size.y) {
        state.map_size = display.size_without_padding() - Point::new(panel_width_tiles, 0);
    }
    assert_eq!(
        display.size_without_padding(),
        (state.map_size.x + panel_width_tiles, state.map_size.y)
    );

    let mut new_keys = new_keys.to_vec();

    if gamepad.south {
        new_keys.push(KeyCode::Enter.into());
    }
    if gamepad.east {
        new_keys.push(KeyCode::Esc.into());
    }
    if gamepad.west {
        new_keys.push(KeyCode::I.into());
    }

    // NOTE: This does add diagonal movement when pressing both D-pad buttons at once.
    //
    // But they need to both be pressed during the same frame or the
    // buttons are treated as separate presses. This can be
    // problematic as sometimes every single button press counts.
    //
    // To deal with that, we'd have to either give the player a few frames buffer (before all the presses are counted). Or keep track of key-down states and only make them effective during key up.
    //
    // The latter slows down the button feel (the game feels slower
    // and less reactive). The former will of course do the same, but
    // may offer a shorter window where the slugginess won't be as
    // stark (at the risk of still missing some double-presses).

    if gamepad.up && gamepad.left {
        log::warn!("Pressed Up and Left!");
        new_keys.push(KeyCode::NumPad7.into());
    } else if gamepad.up && gamepad.right {
        log::warn!("Pressed Up and Right!");
        new_keys.push(KeyCode::NumPad9.into());
    } else if gamepad.down && gamepad.left {
        log::warn!("Pressed Down and Left!");
        new_keys.push(KeyCode::NumPad1.into());
    } else if gamepad.down && gamepad.right {
        log::warn!("Pressed Down and Right!");
        new_keys.push(KeyCode::NumPad3.into());
    } else if gamepad.up {
        new_keys.push(KeyCode::Up.into());
    } else if gamepad.down {
        new_keys.push(KeyCode::Down.into());
    } else if gamepad.left {
        new_keys.push(KeyCode::Left.into());
    } else if gamepad.right {
        new_keys.push(KeyCode::Right.into());
    } else if gamepad.start {
        new_keys.push(KeyCode::Menu.into());
    }

    gamepad.reset_buttons();

    let gamepad_highlighted_tile = if gamepad.left_stick_x > 0.0 && gamepad.left_stick_y > 0.0 {
        Some(Point::new(1, -1))
    } else if gamepad.left_stick_x > 0.0 && gamepad.left_stick_y == 0.0 {
        Some(Point::new(1, 0))
    } else if gamepad.left_stick_x > 0.0 && gamepad.left_stick_y < 0.0 {
        Some(Point::new(1, 1))
    } else if gamepad.left_stick_x < 0.0 && gamepad.left_stick_y > 0.0 {
        Some(Point::new(-1, -1))
    } else if gamepad.left_stick_x < 0.0 && gamepad.left_stick_y == 0.0 {
        Some(Point::new(-1, 0))
    } else if gamepad.left_stick_x < 0.0 && gamepad.left_stick_y < 0.0 {
        Some(Point::new(-1, 1))
    } else if gamepad.left_stick_x == 0.0 && gamepad.left_stick_y > 0.0 {
        Some(Point::new(0, -1))
    } else if gamepad.left_stick_x == 0.0 && gamepad.left_stick_y < 0.0 {
        Some(Point::new(0, 1))
    } else {
        None
    };

    let input = {
        let mut i = Input {
            keys: new_keys.to_vec(),
            mouse,
            tick_id: state.tick_id,
            verification: VerificationWrapper::None,
        };

        if cfg!(feature = "verifications") {
            // NOTE: we're not logging a verification every frame!
            if state.debug {
                i.verification = VerificationWrapper::Verification(state.verification());
            } else {
                let hash = state.verification().hash();
                i.verification = VerificationWrapper::Hash(*hash.as_bytes());
            }
        }
        i
    };

    if state.window_stack.top() == Window::Game && state.player.alive() {
        state::log_input(&mut state.input_logger, input);
        log::debug!(
            "[TICK {}] state.player.pos: {}",
            state.tick_id,
            state.player.pos
        );
    }

    if state.replay && state.player.alive() && !state.paused {
        let replay_input_index = state.tick_id as usize - 1;
        assert_eq!(state.tick_id, state.previous_tick + 1);
        if let Some(input) = state.inputs.get(replay_input_index) {
            assert_eq!(state.tick_id, input.tick_id);

            match &input.verification {
                VerificationWrapper::Verification(expected) => {
                    let actual = state.verification();
                    verify_states(expected, &actual);
                }
                VerificationWrapper::Hash(replay_hash_bytes) => {
                    let replay_hash = blake3::Hash::from_bytes(*replay_hash_bytes);
                    let actual_hash = state.verification().hash();
                    assert_eq!(replay_hash, actual_hash);
                }

                // NOTE: by definition, the lack of a verification is
                // treated as an automatic pass:
                VerificationWrapper::None => (),
            }

            state.keys.extend(input.keys.iter().copied());
            state.mouse = input.mouse;
        } else {
            #[allow(clippy::panic)]
            {
                panic!("Could not load input for {}", state.tick_id);
            }
        }

        #[allow(clippy::panic)]
        if !state.player.alive() && !state.inputs.is_empty() {
            panic!(
                "Game quit too early -- there are still {} \
                         commands queued up.",
                state.inputs.len()
            );
        }
    } else {
        state.keys.extend(new_keys.iter().copied());
        state.mouse = mouse;
    }

    // Quit the game when Q is pressed or on replay and requested
    if ((state.side == Side::Victory || !state.player.alive()) && state.exit_after)
        || (state.replay
            && state.exit_after
            && (state.inputs.is_empty()
                || (!state.player.alive() && state.screen_fading.is_none())))
    {
        show_exit_stats(&state.stats);
        return RunningState::Stopped;
    }

    let simulation_area = formula::simulation_area(state.player.pos);
    let victory_npc_accompanies_player = state.world.monsters(simulation_area).find(|m| {
        m.kind == monster::Kind::Npc
            && m.accompanying_player
            && m.companion_bonus == Some(CompanionBonus::Victory)
    });

    if state.replay
        && state.replay_full_speed
        && state.game_session == GameSession::InProgress
        && state.player.alive()
        && state.side == Side::Player
        && state.window_stack.top() == Window::Game
        && !state.paused
        && state.player.ap() > 0
        && state.explosion_animation.is_none()
        && state.extra_animations.is_empty()
        && state.pos_timer.finished()
        && state.keys.is_empty()
        && victory_npc_accompanies_player.is_none()
    {
        return RunningState::Skip;
    }

    // Full screen on Alt-Enter
    if cfg!(feature = "fullscreen") && state.keys.matches(|k| k.alt && k.code == KeyCode::Enter) {
        log::info!("Pressed Alt+Enter, toggling fullscreen.");
        settings.fullscreen = !settings.fullscreen;
    }

    // Hide the timed message box if it ran out
    let mut window_timed_out = false;
    if let Window::Message { ttl, .. } = state.window_stack.top_mut() {
        if let Some(ttl) = ttl {
            *ttl = util::duration_sub_or_zero(*ttl, dt);
        }
        window_timed_out = ttl.map(|ttl| ttl.as_millis() == 0).unwrap_or(false);
    }
    if window_timed_out {
        state.window_stack.pop();
    }

    // NOTE: Clear the whole screen
    display.clear(state.palette.unexplored_background);

    // // NOTE: This renders the game's icon. Change the tilesize to an
    // // appropriate value.
    // //
    // let origin = Point::new(0, 0);
    // display.set_glyph(origin, 'D', color::depression);
    // display.set_glyph(origin + (1, 0), 'r', color::anxiety);
    // display.set_glyph(origin + (0, 1), '@', color::player);
    // display.set_glyph(origin + (1, 1), 'i', color::dose);
    // display.set_fade(color::BLACK, 1.0);

    // TODO: This might be inefficient for windows fully covering
    // other windows.
    let mut game_update_result = RunningState::Running;

    let screen_rect = egui::Rect::from_min_max(egui::Pos2::ZERO, display.screen_size_px.into());
    ui::egui_root(egui_ctx, screen_rect, |ui| {
        // NOTE: cloning the window list here to let us iterate it and mutate state.
        let windows = state.window_stack.clone();
        for window in windows.windows() {
            let top_level = state.window_stack.top() == *window;
            match window {
                Window::MainMenu => {
                    let active = top_level;
                    game_update_result = main_menu::process(
                        state, ui, gamepad, settings, metrics, display, audio, active,
                    );

                    // Clear any fade set by the gameplay rendering
                    display.fade = color::INVISIBLE;
                }
                Window::Game => {
                    let mut highlighted_tiles = Vec::with_capacity(15);
                    let result = process_game(
                        state,
                        ui,
                        gamepad,
                        settings,
                        metrics,
                        display,
                        audio,
                        dt,
                        fps,
                        top_level,
                        &mut highlighted_tiles,
                        gamepad_highlighted_tile,
                    );
                    render::render_game(state, metrics, display, highlighted_tiles);
                    game_update_result = result;
                }
                Window::Settings => {
                    if top_level {
                        game_update_result = settings::process(
                            state,
                            ui,
                            gamepad,
                            settings,
                            display,
                            audio,
                            settings_store,
                        );
                    }
                    // Clear any fade set by the gameplay rendering
                    display.fade = color::INVISIBLE;
                }
                Window::Help => {
                    if top_level {
                        game_update_result = help::process(state, ui, gamepad, display, audio);
                    }
                    // Clear any fade set by the gameplay rendering
                    display.fade = color::INVISIBLE;
                }
                Window::Endgame => {
                    display.fade = color::INVISIBLE;
                    if top_level {
                        game_update_result = if cfg!(feature = "recording") {
                            crate::windows::call_to_action::process(state, ui, display)
                        } else {
                            endgame::process(
                                state, ui, gamepad, settings, metrics, display, audio, top_level,
                            )
                        };
                    }
                }
                Window::Message { title, message, .. } => {
                    if top_level {
                        game_update_result = message::process(state, ui, title, message, display)
                    }
                }
            }
        }

        if state.replay {
            use egui::widgets::Image;

            let tilesize = crate::graphic::TILE_SIZE as f32;
            let tilemap_width = crate::engine::TILEMAP_TEXTURE_WIDTH as f32;
            let tilemap_height = crate::engine::TILEMAP_TEXTURE_HEIGHT as f32;
            let uv = egui::Rect::from_min_size(
                // NOTE: coordinates
                [110.0 / tilemap_width, 10.0 / tilemap_height].into(),
                // NOTE: the doubling here means draw 2x2 tiles (total of four)
                [
                    (tilesize / tilemap_width) * 2.0,
                    (tilesize / tilemap_height) * 2.0,
                ]
                .into(),
            );

            let image = Image::new(crate::engine::Texture::Tilemap.into(), egui::Vec2::ZERO).uv(uv);
            let mouse_p = egui::Pos2::new(
                state.mouse.screen_pos.x as f32,
                state.mouse.screen_pos.y as f32,
            );

            // NOTE: if this doesn't match the
            // `tilemap_{width,height}` in the `uv` calculations, it
            // will resize the rendered image.
            let image_size = egui::Vec2::new(20.0, 20.0);
            image.paint_at(ui, egui::Rect::from_min_size(mouse_p, image_size));
        }
    });

    // NOTE: process the screen fading animation animation.
    // This must happen outside of the window-custom code because the fadeout could
    // span multiple windows.
    if let Some(mut anim) = state.screen_fading {
        if anim.timer.finished() {
            state.screen_fading = None;
        } else {
            use crate::animation::ScreenFadePhase;
            let prev_phase = anim.phase;
            anim.update(dt);
            let new_phase = anim.phase;
            // TODO: this is a bit hacky, but we want to uncover the screen only
            // after we've faded out:
            if (prev_phase != new_phase)
                && prev_phase == ScreenFadePhase::FadeOut
                && state.show_endscreen_and_uncover_map_during_fadein
            {
                state.uncovered_map = true;
                state.selected_endgame_window_action = Some(endgame::Action::NewGame);
                state.window_stack.push(Window::Endgame);
            }
            state.screen_fading = Some(anim);
        }
    }

    audio.play_mixed_sound_effects(&mut state.audio_rng);

    // NOTE: Clear any unprocessed keys
    while let Some(_key) = state.keys.get() {}

    let update_duration = update_stopwatch.finish();

    if cfg!(feature = "missed-frames") && cfg!(not(feature = "recording")) {
        let expected_ms = 10;
        if update_duration > Duration::from_millis(expected_ms) {
            log::warn!(
                "Game update took too long: {:?} (expected: {}ms)",
                update_duration,
                expected_ms
            );
            display.clear(color::WHITE);
        }
    }

    // TODO: we can no longer sensibly distinguish between building the drawcalls
    // due to egui and having to bundle the update and rendering code.
    let drawcall_stopwatch = Stopwatch::start();
    let drawcall_duration = drawcall_stopwatch.finish();

    if cfg!(feature = "stats") {
        state.stats.push(FrameStats {
            update: update_duration,
            drawcalls: drawcall_duration,
        });
        if state.clock > Duration::new(5, 0) {
            state.stats.update_fps(fps);
        }
    }

    if let RunningState::Stopped = game_update_result {
        if cfg!(feature = "stats") {
            show_exit_stats(&state.stats);
        }
    }

    game_update_result
}

fn enqueue_background_music(audio: &mut Audio, audio_rng: &mut Random) {
    if audio.background_sound_queue.len() <= 1 {
        let sound = if cfg!(feature = "recording") {
            audio.backgrounds.family_breaks.clone()
        } else {
            audio.backgrounds.random(audio_rng)
        };
        if let Some(sound) = sound {
            use rodio::Source;
            let delay = if audio.background_sound_queue.empty() {
                Duration::from_secs(0)
            } else {
                let secs: u64 = audio_rng.range_inclusive(1, 5).try_into().unwrap_or(1);
                Duration::from_secs(secs)
            };
            audio.background_sound_queue.append(sound.delay(delay));
        }
    }
}

fn process_game(
    state: &mut State,
    ui: &mut Ui,
    gamepad: &Gamepad,
    settings: &Settings,
    _metrics: &dyn TextMetrics,
    display: &Display,
    audio: &mut Audio,
    dt: Duration,
    fps: i32,
    active: bool,
    highlighted_tiles: &mut Vec<Point>,
    gamepad_highlighted_tile: Option<Point>,
) -> RunningState {
    use self::sidebar::Action;

    let (mut option, highlighted_tile) =
        sidebar::process(state, ui, gamepad, settings, dt, fps, display, active);
    if let Some(pos) = highlighted_tile {
        highlighted_tiles.push(pos);
    }

    if !active {
        return RunningState::Running;
    }

    if option.is_none() {
        option = if state.keys.matches_code(KeyCode::Esc) || state.keys.matches_code(KeyCode::Menu)
        {
            Some(Action::MainMenu)
        } else if state.keys.matches_code(KeyCode::QuestionMark) {
            Some(Action::Help)
        } else {
            None
        };
    }

    if let Some(
        Action::MainMenu
        | Action::Help
        | Action::UseFood
        | Action::UseDose
        | Action::UseCardinalDose
        | Action::UseDiagonalDose
        | Action::UseStrongDose,
    ) = option
    {
        audio.mix_sound_effect(Effect::Click, Duration::from_millis(0));
    }

    match option {
        Some(Action::MainMenu) => {
            state.window_stack.push(Window::MainMenu);
            return RunningState::Running;
        }
        Some(Action::Help) => {
            state.window_stack.push(Window::Help);
            return RunningState::Running;
        }
        Some(sidebar_action) => {
            let sidebar_command = match sidebar_action {
                Action::UseFood => Some(Command::UseFood),
                Action::UseDose => Some(Command::UseDose),
                Action::UseCardinalDose => Some(Command::UseCardinalDose),
                Action::UseDiagonalDose => Some(Command::UseDiagonalDose),
                Action::UseStrongDose => Some(Command::UseStrongDose),

                Action::MoveN => Some(Command::N),
                Action::MoveS => Some(Command::S),
                Action::MoveW => Some(Command::W),
                Action::MoveE => Some(Command::E),

                Action::MoveNW => Some(Command::NW),
                Action::MoveNE => Some(Command::NE),
                Action::MoveSW => Some(Command::SW),
                Action::MoveSE => Some(Command::SE),

                unexpected_action => {
                    log::warn!("Unexpected sidebar action: {:?}", unexpected_action);
                    None
                }
            };
            if let Some(command) = sidebar_command {
                state.commands.push_back(command);
            }
        }

        None => {}
    }

    // Show the endgame screen on any pressed key:
    if state.game_session == GameSession::Ended
        && state.screen_fading.is_none()
        && (state.keys.matches(|_| true) || state.mouse.right_clicked)
    {
        state.selected_endgame_window_action = Some(endgame::Action::NewGame);
        state.window_stack.push(Window::Endgame);
        return RunningState::Running;
    }

    let player_was_alive = state.player.alive();

    // Uncover map / set the Cheat mode
    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::F6) {
        state.cheating = !state.cheating;
    }

    // NOTE: this will not show up in the replay so that'll be out of
    // sync. We can pass `--invincible` while running the replay
    // though and that should always work, I think.
    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::X) && state.cheating {
        log::info!("Making the player invincible, you cheat!");
        state.player.invincible = true;
    }

    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::F) && state.cheating {
        log::info!("Adding one Food, you cheat!");
        state.player.inventory.push(formula::FOOD_PREFAB);
    }

    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::W) && state.cheating {
        log::info!("Increasing Will by one, you cheat!");
        state.player.will += 1;
    }

    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::D) && state.cheating {
        log::info!("Killing the player");
        state.player.dead = true;
    }

    if cfg!(feature = "cheating") && state.keys.matches_code(KeyCode::V) && state.cheating {
        place_victory_npc(state);
    }

    // TODO: NOTE: this now doesn't work on replays because state.keys only contains the replay keys
    let paused_one_step = state.paused && state.keys.matches_code(KeyCode::Right);

    // Animation to re-center the screen around the player when they
    // get too close to an edge.
    state.pos_timer.update(dt);
    if state.pos_timer.finished() {
        state.screen_position_in_world = state.new_screen_pos;
        state.offset_px = Point::zero();
    } else {
        let tilesize = settings.tile_size as f32;
        let percentage = util::sine_curve(state.pos_timer.percentage_elapsed());
        let x = ((state.old_screen_pos.x - state.new_screen_pos.x) as f32) * percentage * tilesize;
        let y = ((state.old_screen_pos.y - state.new_screen_pos.y) as f32) * percentage * tilesize;
        state.offset_px = Point::new(x as i32, y as i32);
    }

    let mut entire_turn_ended = false;

    let simulation_area = formula::simulation_area(state.player.pos);

    if (!state.paused || paused_one_step) && state.side != Side::Victory {
        let monster_count = state.world.monsters(simulation_area).count();
        let monster_with_ap_count = state
            .world
            .monsters(simulation_area)
            .filter(|m| m.has_ap(1))
            .count();
        let monster_cumulative_ap: i32 = state
            .world
            .monsters(simulation_area)
            .map(|m| m.ap.to_int())
            .sum();
        log::debug!(
            "Player AP: {}, monsters: {}, active mon: {}, total mon AP: {}",
            state.player.ap(),
            monster_count,
            monster_with_ap_count,
            monster_cumulative_ap
        );

        if state.inventory_focused {
            // Don't turn keys into commands. The player should not be
            // controllable while the inventory is active.

            // TODO: we need to do the same for mouse. Or make sure we
            // close inventory on click at least.
        } else {
            process_keys(&mut state.keys, &mut state.commands);
        }

        if state.mouse.left_clicked || state.mouse.right_clicked {
            state.inventory_focused = false;
        }

        if state.mouse.left_clicked || state.commands.front() == Some(&Command::WalkPath) {
            state.path_walking_timer.finish();
        } else {
            state.path_walking_timer.update(dt);
        }

        // NOTE: Show the path from the player to the mouse pointer
        let mouse_inside_map =
            state.mouse.tile_pos >= (0, 0) && state.mouse.tile_pos < state.map_size;

        let visible = state.mouse_world_position().inside_circular_area(
            state.player.pos,
            formula::exploration_radius(state.player.mind),
        );

        if state.game_session.started() && state.player.alive() && !state.inventory_focused {
            let source = state.player.pos;

            if let Some(destination_offset) = gamepad_highlighted_tile {
                let destination = source + destination_offset;
                let check_irresistible = true;
                let path = pathfinding::Path::find(
                    source,
                    destination,
                    &state.world,
                    Blocker::WALL,
                    state.player.pos,
                    state.player.will.to_int(),
                    check_irresistible,
                    formula::PATHFINDING_PLAYER_MOUSE_LIMIT,
                    &pathfinding::player_cost,
                );
                for point in path.clone() {
                    let screen_pos = state.screen_pos_from_world_pos(point);
                    highlighted_tiles.push(screen_pos);
                }
                state.player_path = path;
            } else if mouse_inside_map && visible {
                let destination = state.mouse_world_position();
                let check_irresistible = true;
                let path = pathfinding::Path::find(
                    source,
                    destination,
                    &state.world,
                    Blocker::WALL,
                    state.player.pos,
                    state.player.will.to_int(),
                    check_irresistible,
                    formula::PATHFINDING_PLAYER_MOUSE_LIMIT,
                    &pathfinding::player_cost,
                );
                for point in path.clone() {
                    let screen_pos = state.screen_pos_from_world_pos(point);
                    highlighted_tiles.push(screen_pos);
                }
                state.player_path = path;
            } else {
                state.player_path.clear();
            }
        }

        state.player.motion_animation.update(dt);
        for monster in state.world.monsters_mut(simulation_area) {
            monster.motion_animation.update(dt);
        }
        for motion_animation in &mut state.extra_animations {
            motion_animation.animation.update(dt);
        }

        // NOTE: Process 1 action point of the player and then 1 AP of
        // all monsters. This means that their turns will alternate.
        // E.g. if the player has 2 APs and they're close to a
        // Depression, the player will move 1 turn first, then
        // Depression 1 etc.

        let player_ap = state.player.ap();
        log::debug!("Player AP before processing: {player_ap}");
        if state.player.ap() >= 1 {
            process_player(state, display, audio, simulation_area);
        }
        let player_took_action = player_ap > state.player.ap();
        let monsters_can_move = state.player.ap() == 0 || player_took_action;
        log::debug!("Player AP: {player_ap}, Player took action: {player_took_action}, Monsters can move: {monsters_can_move}");

        if state.explosion_animation.is_none() {
            if monsters_can_move {
                process_monsters(
                    &mut state.world,
                    &mut state.player,
                    simulation_area,
                    display.tile_size,
                    &mut state.rng,
                    &mut state.audio_rng,
                    audio,
                    &state.palette,
                    &mut state.extra_animations,
                );
            } else {
                log::debug!("Monsters waiting for player.");
            }
        } else {
            log::debug!("Monster's waiting for the explosion to end.");
        }

        // NOTE: the anxiety counter bar is hidden at the start, but
        // we want to show it as soon as it increases.
        if player_took_action && !state.player.anxiety_counter.is_min() {
            state.show_anxiety_counter = true;
        }

        if player_took_action && state.player.mind.is_high() {
            if let Some(victory_npc_id) = state.victory_npc_id.take() {
                log::info!("Player got High, the Victory NPC disappears!");
                if let Some(vnpc) = state.world.monster_mut(victory_npc_id) {
                    // TODO: move this (and other init stuff from
                    // Monster::new) to custom functions?
                    vnpc.kind = monster::Kind::Signpost;
                    vnpc.behavior = ai::Behavior::Immobile;
                    vnpc.ai_state = ai::AIState::NoOp
                }
            }
        }

        // Reset all action points only after everyone is at zero:
        let player_turn_ended = !state.player.has_ap(1);
        let monster_turn_ended = state
            .world
            .monsters(simulation_area)
            .filter(|m| m.has_ap(1))
            .count()
            == 0;

        entire_turn_ended = player_turn_ended && monster_turn_ended;
        log::debug!(
            "Entire turn ended: {}, player turn ended: {}, monster turn ended: {}",
            entire_turn_ended,
            player_turn_ended,
            monster_turn_ended
        );
    }

    // Reset the player & monster action points
    // NOTE: doing this only after we've logged the validations. Actually maybe we want to do this
    // before we start turn processing??
    if entire_turn_ended {
        log::debug!("Starting new turn for player and monsters.");
        state.player.new_turn();
        for monster in state.world.monsters_mut(simulation_area) {
            monster.new_turn();
        }
    }

    if entire_turn_ended {
        log::debug!("Turn {} has ended.", state.turn);
        state.turn += 1;
    }

    // NOTE: Load up new chunks if necessary
    if entire_turn_ended {
        for pos in simulation_area.points() {
            state.world.ensure_chunk_at_pos(pos);
        }
    }

    // Run the dose explosion effect here:
    if let Some(ref anim) = state.explosion_animation {
        for (pos, _, effect) in anim.tiles() {
            if effect.contains(animation::TileEffect::KILL) {
                kill_monster(pos, &mut state.world, audio);
            }
            if effect.contains(animation::TileEffect::SHATTER) {
                if let Some(cell) = state.world.cell_mut(pos) {
                    cell.tile.kind = TileKind::Empty;
                    cell.tile.graphic = Graphic::Empty;
                    cell.items.clear();
                }
            }
        }
    }

    // Set the fadeout animation on death
    if player_was_alive && !state.player.alive() {
        use crate::player::CauseOfDeath::*;
        log::info!("Player died.");

        audio.mix_sound_effect(Effect::GameOver, Duration::from_millis(0));
        let cause_of_death = formula::cause_of_death(&state.player);
        let fade_color = if cfg!(feature = "recording") {
            state.palette.fade_to_black_animation
        } else {
            match cause_of_death {
                Some(Exhausted) => state.palette.exhaustion_animation,
                Some(Overdosed) => state.palette.overdose_animation,
                Some(_) => state.palette.death_animation,
                None => {
                    // NOTE: this shouldn't happen (there should always be
                    // a cause of death) but if it did, we won't crash
                    state.palette.death_animation
                }
            }
        };

        let fade = formula::mind_fade_value(state.player.mind);

        let fade_out_ms = if cfg!(feature = "recording") {
            2500
        } else if state.replay_full_speed {
            500
        } else {
            2500
        };

        let fade_in_ms = if cfg!(feature = "recording") {
            1300
        } else if state.replay_full_speed {
            200
        } else {
            500
        };

        let initial_fade_percentage = 1.0 - fade;
        if state.challenge.one_chance {
            state.game_session = GameSession::Ended;
            state.show_endscreen_and_uncover_map_during_fadein = true;
            log::debug!("Game real time: {:?}", state.clock);
        } else {
            // NOTE: Don't die, reset the player to the initial state instead:
            state.player.reset();
        }
        state.screen_fading = Some(animation::ScreenFade::new(
            fade_color,
            Duration::from_millis(fade_out_ms),
            Duration::from_millis(200),
            Duration::from_millis(fade_in_ms),
            initial_fade_percentage,
        ));
    }

    let explored = state
        .world
        .cell(state.mouse_world_position())
        .is_none_or(|cell| cell.explored);

    let mouse_window_pos_px = state.mouse.screen_pos;
    let window_size_px = display.screen_size_px;
    let sidebar_width_px = formula::sidebar_width_px(display.text_size);
    let game_area_px = Point::new(window_size_px.x - sidebar_width_px, window_size_px.y);

    // NOTE: Is the mouse pointer inside the actual game map? Otherwise it's in the sidebar or something.
    let pointer_inside_game_area = mouse_window_pos_px.x >= 0
        && mouse_window_pos_px.y >= 0
        && mouse_window_pos_px.x <= game_area_px.x
        && mouse_window_pos_px.y <= game_area_px.y;

    // NOTE: show tooltip of a hovered-over object
    let tooltip = if !explored && settings.hide_unseen_tiles || !pointer_inside_game_area {
        None
    } else if state.mouse_world_position() == state.player.pos {
        Some("Player Character")
    } else if let Some(monster) = state.world.monster_on_pos(state.mouse_world_position()) {
        Some(monster.name())
    } else if let Some(cell) = state.world.cell(state.mouse_world_position()) {
        cell.items.first().map(|item| item.kind.name())
    } else {
        None
    };
    if let Some(tooltip) = tooltip {
        // NOTE: only show tooltips when we're not scrolling the screen.
        // It looks bad otherwise.
        if state.pos_timer.finished() {
            egui::show_tooltip_text(ui.ctx(), egui::Id::new("Tile Tooltip"), tooltip);
        }
    }

    // NOTE: update the dose/food explosion animations
    state.explosion_animation = state.explosion_animation.take().and_then(|mut animation| {
        animation.update(dt);
        if animation.finished() {
            None
        } else {
            Some(animation)
        }
    });

    // NOTE: re-centre the display if the player reached the end of the screen
    let no_left_mouse = !state.mouse.left_is_down && !state.mouse.left_clicked;
    if state.pos_timer.finished() && no_left_mouse {
        let display_pos = state.screen_pos_from_world_pos(state.player.pos);
        // NOTE: this is the re-center speed. We calculate it based on
        // the map size. That way the speed itself remains more or
        // less constant.
        let scroll_rate_ms_per_tile = 14;
        let max_display_size = std::cmp::max(state.map_size.x, state.map_size.y) as u64;
        let ms = if state.replay_full_speed {
            100
        } else {
            scroll_rate_ms_per_tile * max_display_size
        };
        let dur = Duration::from_millis(ms);
        let exploration_radius = formula::exploration_radius(state.player.mind);
        // TODO: move the screen roughly the same distance along X and Y
        if display_pos.x < exploration_radius
            || display_pos.x >= state.map_size.x - exploration_radius
            || display_pos.y < exploration_radius
            || display_pos.y >= state.map_size.y - exploration_radius
        {
            state.pos_timer = Timer::new(dur);
            state.old_screen_pos = state.screen_position_in_world;
            // change the screen centre to that of the player
            state.new_screen_pos = state.player.pos;
        } else {
            // Do nothing
        }
    }
    // Hide the keyboard movement hints if the player gets too close
    {
        // NOTE: this is no longer having any effect. Hints are disabled in `State::new`.

        let player_screen_pos = state.screen_pos_from_world_pos(state.player.pos);
        let d = 10;
        if player_screen_pos.x < d
            || player_screen_pos.y < d
            || state.map_size.x - player_screen_pos.x < d
            || state.map_size.y - player_screen_pos.y < d
        {
            state.show_keyboard_movement_hints = false;
        }

        if cfg!(feature = "recording") {
            state.show_keyboard_movement_hints = false;
        }
    }

    // NOTE: Remove any animations that are already finished.
    state.extra_animations.retain(|a| a.animation.in_progress());
    RunningState::Running
}

fn process_monsters(
    world: &mut World,
    player: &mut player::Player,
    area: Rectangle,
    tile_size: i32,
    rng: &mut Random,
    audio_rng: &mut Random,
    audio: &mut Audio,
    palette: &Palette,
    extra_animations: &mut Vec<MotionAnimation>,
) {
    if !player.alive() {
        return;
    }
    log::debug!("Processing monsters");

    // NOTE: one quarter of the map area should be a decent overestimate
    let monster_count_estimate = area.size().x * area.size().y / 4;
    assert!(monster_count_estimate > 0);
    let mut monster_positions_vec = world
        .monsters(area)
        .filter(|m| m.has_ap(1))
        .map(|m| (m.ap.to_int(), m.position))
        .collect::<Vec<_>>();
    // NOTE: `world.monsters` does not give a stable result so we need to sort
    // it here to ensure correct replays.
    // NOTE: there's always at most one monster at a given position so this should always produce
    // the same ordering.
    //
    // We sort by action points (so depression always goes first), by
    // distance to player (so a closer monster can move first and make
    // space for another one near by) and then by coordinates just to
    // have some awy to always produce a stable ordering.
    monster_positions_vec
        .sort_by_key(|&(ap, pos)| (ap, player.pos.distance(pos) as i32, pos.x, pos.y));
    let mut monster_positions_to_process: VecDeque<_> = monster_positions_vec.into();
    log::debug!(
        "Monsters to process: {}",
        monster_positions_to_process.len()
    );

    while let Some((_, monster_position)) = monster_positions_to_process.pop_front() {
        if !player.alive() {
            // Don't process any new monsters if the player's dead
            // because we want the game to stop and freeze at that
            // time.
            //
            // But we still want all the effects (e.g. attack
            // animatino) that started to finish so we don't want to
            // quit the loop as soon as the player dies.
            continue;
        }
        if let Some(monster_readonly) = world.monster_on_pos(monster_position).cloned() {
            let action = {
                let (update, action) = monster_readonly.act(player.info(), world, rng);
                if let Some(monster) = world.monster_on_pos(monster_position) {
                    monster.ai_state = update.ai_state;
                    monster.ap = Ranged::new(
                        monster.ap.to_int(),
                        InclusiveRange(monster.ap.min(), update.max_ap),
                    );

                    monster.spend_ap(1);
                }
                action
            };

            let (animated_monster_position, animation) = match action {
                Action::Move(destination) => {
                    assert_eq!(monster_position, monster_readonly.position);

                    let pos = monster_readonly.position;

                    let path_changed = monster_readonly
                        .path
                        .last()
                        .is_none_or(|&cached_destination| cached_destination != destination);

                    // NOTE: we keep a cache of any previously calculated
                    // path in `monster.path`. If the precalculated path
                    // is blocked or there is none, calculate a new one
                    // and cache it. Otherwise, just walk it.
                    let (newpos, newpath) = if monster_readonly.path.is_empty()
                        || path_changed
                        || !world.walkable(
                            monster_readonly.path[0],
                            monster_readonly.blockers,
                            player.pos,
                        ) {
                        // Calculate a new path or recalculate the existing one.
                        let check_irresistible = false;
                        let mut path = pathfinding::Path::find(
                            pos,
                            destination,
                            world,
                            monster_readonly.blockers,
                            player.pos,
                            player.will.to_int(),
                            check_irresistible,
                            formula::PATHFINDING_MONSTER_LIMIT,
                            &pathfinding::monster_cost,
                        );
                        let newpos = path.next().unwrap_or(pos);
                        // Cache the path-finding result
                        let newpath = path.collect();
                        (newpos, newpath)
                    } else {
                        (monster_readonly.path[0], monster_readonly.path[1..].into())
                    };

                    world.move_monster(pos, newpos, player.pos);
                    let monster_visible = newpos
                        .inside_circular_area(player.pos, formula::exploration_radius(player.mind));
                    if monster_visible {
                        let delay = audio.random_delay(audio_rng);
                        audio.mix_sound_effect(Effect::MonsterMoved, delay);
                    }
                    if let Some(monster) = world.monster_on_pos(newpos) {
                        monster.path = newpath;
                        if monster.has_ap(1) {
                            monster.trail = Some(newpos);
                        }
                    }

                    let anim = animation::Move::ease(
                        pos * tile_size,
                        newpos * tile_size,
                        formula::ANIMATION_MOVE_DURATION,
                    );
                    assert_eq!(anim.finished(), false);
                    (newpos, anim)
                }

                Action::Attack(target_pos, damage) => {
                    assert_eq!(target_pos, player.pos);
                    player.take_effect(damage);
                    audio.mix_sound_effect(Effect::PlayerHit, Duration::from_millis(0));

                    let anim = animation::Move::bounce(
                        monster_readonly.position * (tile_size / 3),
                        target_pos * (tile_size / 3),
                        formula::ANIMATION_ATTACK_DURATION,
                    );

                    if monster_readonly.die_after_attack {
                        kill_monster(monster_readonly.position, world, audio);
                        extra_animations.push(MotionAnimation {
                            pos: monster_readonly.position,
                            graphic: monster_readonly.graphic(),
                            color: monster_readonly.color(palette),
                            animation: anim.clone(),
                        });
                    }
                    if !player.alive() {
                        // NOTE: this monster killed the player, set the perpetrator.
                        player.perpetrator = Some(monster_readonly.clone());
                    }

                    (monster_readonly.position, anim)
                }

                Action::Use(_) => {
                    log::error!("Trying to run the Use action on a monster. That's not defined!");
                    (monster_readonly.position, animation::Move::none())
                }
            };

            if let Some(monster) = world.monster_on_pos(animated_monster_position) {
                monster.motion_animation = animation;
            }
        }
    }
}

fn process_player_action(
    player: &mut player::Player,
    commands: &mut VecDeque<Command>,
    world: &mut World,
    simulation_area: Rectangle,
    explosion_animation: &mut Option<Box<dyn AreaOfEffect>>,
    rng: &mut Random,
    window_stack: &mut crate::windows::Windows<Window>,
    bumped_into_a_monster: &mut bool,
    tile_size: i32,
    palette: &Palette,
    audio: &mut Audio,
) {
    log::debug!("Processing player action");
    if !player.alive() {
        log::debug!("Processing player action, but the player is dead.");
        return;
    }
    if !player.has_ap(1) {
        log::debug!(
            "Processing player action, but the player has no AP: {}",
            player.ap()
        );
        return;
    }

    if let Some(command) = commands.pop_front() {
        log::debug!("Player Command: {:?}", command);
        let mut action = match command {
            Command::N => Action::Move(player.pos + (0, -1)),
            Command::S => Action::Move(player.pos + (0, 1)),
            Command::W => Action::Move(player.pos + (-1, 0)),
            Command::E => Action::Move(player.pos + (1, 0)),

            Command::NW => Action::Move(player.pos + (-1, -1)),
            Command::NE => Action::Move(player.pos + (1, -1)),
            Command::SW => Action::Move(player.pos + (-1, 1)),
            Command::SE => Action::Move(player.pos + (1, 1)),

            Command::UseFood => Action::Use(item::Kind::Food),
            Command::UseDose => Action::Use(item::Kind::Dose),
            Command::UseCardinalDose => Action::Use(item::Kind::CardinalDose),
            Command::UseDiagonalDose => Action::Use(item::Kind::DiagonalDose),
            Command::UseStrongDose => Action::Use(item::Kind::StrongDose),

            // NOTE: ignore, this has been processed elsewhere
            Command::WalkPath => return,

            Command::ShowMessageBox {
                ttl,
                title,
                message,
            } => {
                window_stack.push(window::timed_message_box(title, message, ttl));
                return;
            }
        };
        log::debug!("Action from Command: {:?}", action);

        if player.stun.to_int() > 0 {
            action = Action::Move(player.pos);
        } else if player.panic.to_int() > 0 {
            let new_pos =
                world.random_neighbour_position(rng, player.pos, Blocker::WALL, player.pos);
            action = Action::Move(new_pos);
        } else if let Some((dose_pos, dose)) = world.nearest_dose(player.pos, 5) {
            let resist_radius =
                formula::player_resist_radius(dose.irresistible, player.will.to_int()) as usize;
            if player.pos.tile_distance(dose_pos) < resist_radius as i32 {
                // We're already in the resist radius so we don't care about the cost.
                let check_irresistible = false;
                let mut path = pathfinding::Path::find(
                    player.pos,
                    dose_pos,
                    world,
                    Blocker::WALL,
                    player.pos,
                    player.will.to_int(),
                    check_irresistible,
                    formula::PATHFINDING_DOSE_RESIST_LIMIT,
                    &pathfinding::direct_cost,
                );

                let new_pos_opt = if path.len() <= resist_radius {
                    path.next()
                } else {
                    None
                };

                if let Some(new_pos) = new_pos_opt {
                    action = Action::Move(new_pos);
                } else {
                    // NOTE: no path leading to the irresistible dose
                }
            }
        }

        // NOTE: If we have doses in the inventory that we wouldn't be
        // able to pick up anymore, use them up one by one each turn:
        let carried_irresistible_dose = player
            .inventory
            .iter()
            .find(|i| {
                i.is_dose()
                    && formula::player_resist_radius(i.irresistible, player.will.to_int()) > 0
            })
            .map(|i| i.kind);
        if let Some(kind) = carried_irresistible_dose {
            action = Action::Use(kind);
        }

        log::debug!("Final Action: {:?}", action);
        match action {
            Action::Move(dest) => {
                let dest_walkable =
                    world.walkable(dest, Blocker::WALL | Blocker::MONSTER, player.pos);
                let bumping_into_monster = world.monster_on_pos(dest).is_some();
                if bumping_into_monster {
                    player.spend_ap(1);
                    // info!("Player attacks {:?}", monster);
                    player.motion_animation = animation::Move::bounce(
                        player.pos * (tile_size / 3),
                        dest * (tile_size / 3),
                        formula::ANIMATION_ATTACK_DURATION,
                    );
                    if let Some(kind) = world.monster_on_pos(dest).map(|m| m.kind) {
                        match kind {
                            monster::Kind::Anxiety => {
                                log::debug!(
                                    "Bumped into anxiety! Current anxiety counter: {:?}",
                                    player.anxiety_counter
                                );
                                let increment =
                                    if player.bonuses.contains(&CompanionBonus::DoubleWillGrowth) {
                                        2
                                    } else {
                                        1
                                    };
                                log::debug!("Anxiety increment: {:?}", increment);
                                player.anxiety_counter += increment;
                                log::debug!("New anxiety counter: {:?}", player.anxiety_counter);
                                if player.anxiety_counter.is_max() {
                                    log::info!("Increasing player's will");
                                    player.will += 1;
                                    player.anxiety_counter.set_to_min();
                                }
                            }
                            monster::Kind::Hunger => {
                                let modifier = Modifier::Attribute {
                                    state_of_mind: 3,
                                    will: 0,
                                };
                                player.take_effect(modifier);
                            }
                            // NOTE: NPCs don't give bonuses or accompany the player when high.
                            monster::Kind::Npc if player.mind.is_sober() => {
                                if let Some(monster) = world.monster_on_pos(dest) {
                                    log::info!("Bumped into NPC: {}", monster);
                                }

                                // Clear any existing monsters accompanying the player. The player
                                // can have only one companion at a time right now.
                                //
                                // TODO: it also sounds like we could just track the followers in
                                // the Player/State struct but that needs Monster IDs.
                                let npcs = world
                                    .monsters_mut(simulation_area)
                                    .filter(|m| m.kind == monster::Kind::Npc);
                                for npc in npcs {
                                    if npc.position == dest {
                                        log::info!("NPC {} accompanies the player.", npc);
                                        npc.accompanying_player = true;
                                        assert!(npc.companion_bonus.is_some());
                                    } else if npc.accompanying_player {
                                        log::info!("NPC {} leaves the player.", npc);
                                        npc.accompanying_player = false;
                                    }
                                }
                            }

                            monster::Kind::Signpost => {
                                log::info!("Bumped into a signpost!");
                                window_stack.push(
                                    window::message_box(
                                        "Message",
                                        "\"I thought you were going to stay sober for good. I was wrong. Goodbye.\""));
                            }

                            _ => {}
                        }
                        kill_monster(dest, world, audio);

                        if kind.is_monster() {
                            *bumped_into_a_monster = true;
                        }
                    }
                } else if dest_walkable {
                    player.spend_ap(1);
                    player.motion_animation = animation::Move::ease(
                        player.pos * tile_size,
                        dest * tile_size,
                        formula::ANIMATION_MOVE_DURATION,
                    );
                    player.move_to(dest);
                    audio.mix_sound_effect(Effect::Walk, Duration::from_millis(0));
                    while let Some(item) = world.pickup_item(dest) {
                        use crate::item::Kind::*;
                        match item.kind {
                            Food => player.inventory.push(item),
                            Dose | StrongDose | CardinalDose | DiagonalDose => {
                                let resist_radius = formula::player_resist_radius(
                                    item.irresistible,
                                    player.will.to_int(),
                                );
                                if resist_radius == 0 {
                                    player.inventory.push(item);
                                } else {
                                    use_dose(player, explosion_animation, item, palette, audio);
                                }
                            }
                        }
                    }
                } else {
                    // NOTE: we bumped into a wall, don't do anything
                }
            }

            Action::Use(item::Kind::Food) => {
                if let Some(food_idx) = player
                    .inventory
                    .iter()
                    .position(|&i| i.kind == item::Kind::Food)
                {
                    player.spend_ap(1);
                    audio.mix_sound_effect(Effect::Explosion, Duration::from_millis(0));
                    let food = player.inventory.remove(food_idx);
                    player.take_effect(food.modifier);
                    let food_explosion_radius = 2;
                    let animation = animation::SquareExplosion::new(
                        player.pos,
                        food_explosion_radius,
                        1,
                        palette.explosion,
                    );
                    *explosion_animation = Some(Box::new(animation));
                }
            }

            Action::Use(item::Kind::Dose) => {
                if let Some(dose_index) = player
                    .inventory
                    .iter()
                    .position(|&i| i.kind == item::Kind::Dose)
                {
                    player.spend_ap(1);
                    let dose = player.inventory.remove(dose_index);
                    use_dose(player, explosion_animation, dose, palette, audio);
                }
            }

            Action::Use(item::Kind::StrongDose) => {
                if let Some(dose_index) = player
                    .inventory
                    .iter()
                    .position(|&i| i.kind == item::Kind::StrongDose)
                {
                    player.spend_ap(1);
                    let dose = player.inventory.remove(dose_index);
                    use_dose(player, explosion_animation, dose, palette, audio);
                }
            }

            Action::Use(item::Kind::CardinalDose) => {
                if let Some(dose_index) = player
                    .inventory
                    .iter()
                    .position(|&i| i.kind == item::Kind::CardinalDose)
                {
                    player.spend_ap(1);
                    let dose = player.inventory.remove(dose_index);
                    use_dose(player, explosion_animation, dose, palette, audio);
                }
            }

            Action::Use(item::Kind::DiagonalDose) => {
                if let Some(dose_index) = player
                    .inventory
                    .iter()
                    .position(|&i| i.kind == item::Kind::DiagonalDose)
                {
                    player.spend_ap(1);
                    let dose = player.inventory.remove(dose_index);
                    use_dose(player, explosion_animation, dose, palette, audio);
                }
            }

            Action::Attack(_, _) => {
                unreachable!();
            }
        }
    } else {
        log::debug!("No Command found");
    }
}

fn process_player(
    state: &mut State,
    display: &Display,
    audio: &mut Audio,
    simulation_area: Rectangle,
) {
    {
        log::debug!("Processing player");

        // appease borrowck
        let player = &mut state.player;

        // NPCs should unfollow an intoxicated player:
        if player.mind.is_high() {
            let npcs = state.world.monsters_mut(simulation_area).filter(|m| {
                m.kind == monster::Kind::Npc && m.accompanying_player && m.companion_bonus.is_some()
            });
            for npc in npcs {
                log::info!("{:?} will not accompany an intoxicated player.", npc);
                npc.accompanying_player = false;
            }
        }

        let world = &state.world;

        // NOTE: if the player manages to outrun the NPC (they follow
        // the player but it still can happen), the bonus will
        // disappear.
        let npc_bonuses = world
            .monsters(simulation_area)
            .filter(|m| {
                m.kind == monster::Kind::Npc && m.accompanying_player && m.companion_bonus.is_some()
            })
            .map(|m| {
                // NOTE: this unwrap should always succeed due to the
                // filter check above. Providing a fallback to prevent
                // any crasches caused by future refactoring.
                m.companion_bonus.unwrap_or_else(|| {
                    log::error!(
                        "Trying to get a companion bonus where one doesn't exist, but  it should."
                    );
                    CompanionBonus::DoubleWillGrowth
                })
            });
        player.bonuses.clear();
        player.bonuses.extend(npc_bonuses);
    }

    let visible = state.mouse_world_position().inside_circular_area(
        state.player.pos,
        formula::exploration_radius(state.player.mind),
    );

    log::debug!(
        "left down: {}, visible: {}, walking timer done: {}",
        state.mouse.left_is_down,
        visible,
        state.path_walking_timer.finished()
    );
    log::debug!("Player path: {:?}", state.player_path);

    let walk_the_path_command = state.commands.front() == Some(&Command::WalkPath);
    if walk_the_path_command {
        state.commands.pop_front();
    }

    // NOTE: If the player is following a path move them one step along the path
    if (walk_the_path_command || (state.mouse.left_is_down && visible))
        && state.path_walking_timer.finished()
    {
        state.path_walking_timer.reset();
        if let Some(destination) = state.player_path.next() {
            let command = match destination - state.player.pos {
                Point { x: 0, y: -1 } => Some(Command::N),
                Point { x: 0, y: 1 } => Some(Command::S),
                Point { x: -1, y: 0 } => Some(Command::W),
                Point { x: 1, y: 0 } => Some(Command::E),

                Point { x: -1, y: -1 } => Some(Command::NW),
                Point { x: -1, y: 1 } => Some(Command::SW),
                Point { x: 1, y: -1 } => Some(Command::NE),
                Point { x: 1, y: 1 } => Some(Command::SE),

                unexpected => {
                    log::debug!("Unexpected command point: {:?}", unexpected);
                    None
                }
            };
            if let Some(command) = command {
                log::debug!("Pushing mouse trail command: {:?}", command);
                state.commands.push_front(command);
            } else {
                log::debug!("We're in the player path section, but no command is pushed.");
            }
        }
    }

    log::debug!("Commands: {:?}", state.commands);
    let previous_action_points = state.player.ap();
    process_player_action(
        &mut state.player,
        &mut state.commands,
        &mut state.world,
        simulation_area,
        &mut state.explosion_animation,
        &mut state.rng,
        &mut state.window_stack,
        &mut state.player_bumped_into_a_monster,
        display.tile_size,
        &state.palette,
        audio,
    );
    log::debug!("player action processed");

    // If the player ever picks up a dose, mark it in this variable:
    let player_picked_up_a_dose = state.player.inventory.iter().any(item::Item::is_dose);
    if player_picked_up_a_dose {
        state.player_picked_up_a_dose = true;
    }
    log::debug!("Player picked up a dose: {}", player_picked_up_a_dose);

    let spent_ap_this_turn = previous_action_points > state.player.ap();
    log::debug!("Player spent AP this turn: {}", spent_ap_this_turn);

    // Place the Victory NPC if the player behaved themself.
    if state.player.will.is_max() && !state.player.mind.is_high() && state.victory_npc_id.is_none()
    {
        place_victory_npc(state);
    }

    // Set the longest high streak
    if spent_ap_this_turn {
        if state.player.mind.is_high() {
            state.player.current_high_streak += 1;
            if state.player.current_high_streak > state.player.longest_high_streak {
                state.player.longest_high_streak = state.player.current_high_streak;
            }
        } else {
            state.player.current_high_streak = 0;
        }
    }

    // NOTE: The player has reached the Victory NPC. Win the game! \o/
    if state.player.bonuses.contains(&CompanionBonus::Victory) {
        win_the_game(state);
    }

    state.world.explore(
        state.player.pos,
        formula::exploration_radius(state.player.mind),
    );
}

fn process_keys(keys: &mut Keys, commands: &mut VecDeque<Command>) {
    use crate::keys::KeyCode::*;
    while let Some(key) = keys.get() {
        let command = match key {
            // Numpad (8246 for cardinal and 7193 for diagonal movement)
            Key { code: NumPad8, .. } => Some(Command::N),
            Key { code: NumPad2, .. } => Some(Command::S),
            Key { code: NumPad4, .. } => Some(Command::W),
            Key { code: NumPad6, .. } => Some(Command::E),
            Key { code: NumPad7, .. } => Some(Command::NW),
            Key { code: NumPad1, .. } => Some(Command::SW),
            Key { code: NumPad9, .. } => Some(Command::NE),
            Key { code: NumPad3, .. } => Some(Command::SE),

            // NotEye (arrow keys plus Ctrl and Shift modifiers for
            // horizontal movement)
            Key { code: Up, .. } => Some(Command::N),
            Key { code: Down, .. } => Some(Command::S),
            Key {
                code: Left,
                ctrl: false,
                shift: true,
                ..
            } => Some(Command::NW),
            Key {
                code: Left,
                ctrl: true,
                shift: false,
                ..
            } => Some(Command::SW),
            Key {
                code: Left,
                alt: true,
                shift: false,
                ..
            } => Some(Command::SW),
            Key {
                code: Left,
                logo: true,
                shift: false,
                ..
            } => Some(Command::SW),
            Key { code: Left, .. } => Some(Command::W),
            Key {
                code: Right,
                ctrl: false,
                shift: true,
                ..
            } => Some(Command::NE),
            Key {
                code: Right,
                ctrl: true,
                shift: false,
                ..
            } => Some(Command::SE),
            Key {
                code: Right,
                alt: true,
                shift: false,
                ..
            } => Some(Command::SE),
            Key {
                code: Right,
                logo: true,
                shift: false,
                ..
            } => Some(Command::SE),
            Key { code: Right, .. } => Some(Command::E),

            // Vi keys (hjkl for cardinal and yubn for diagonal movement)
            Key { code: K, .. } => Some(Command::N),
            Key { code: J, .. } => Some(Command::S),
            Key { code: H, .. } => Some(Command::W),
            Key { code: L, .. } => Some(Command::E),
            Key { code: Y, .. } => Some(Command::NW),
            Key { code: B, .. } => Some(Command::SW),
            Key { code: U, .. } => Some(Command::NE),
            Key { code: N, .. } => Some(Command::SE),

            // Non-movement commands
            Key { code: E, .. } => Some(Command::UseFood),

            Key { code: Enter, .. } => Some(Command::WalkPath),

            _ => inventory_commands(key),
        };
        if let Some(command) = command {
            log::debug!("Pushing Command: {:?} from Key: {:?}", command, key);
            commands.push_back(command);
        }
    }
}

fn inventory_commands(key: Key) -> Option<Command> {
    use crate::{item::Kind, keys::KeyCode::*};

    for kind in Kind::iter() {
        let num_key = match inventory_key(kind) {
            1 => D1,
            2 => D2,
            3 => D3,
            4 => D4,
            5 => D5,
            6 => D6,
            7 => D7,
            8 => D8,
            9 => D9,
            _ => unreachable!("There should only ever be 9 item kinds at most."),
        };

        if key.code == num_key {
            let command = match kind {
                Kind::Food => Command::UseFood,
                Kind::Dose => Command::UseDose,
                Kind::CardinalDose => Command::UseCardinalDose,
                Kind::DiagonalDose => Command::UseDiagonalDose,
                Kind::StrongDose => Command::UseStrongDose,
            };
            return Some(command);
        }
    }
    None
}

pub fn inventory_key(kind: item::Kind) -> u8 {
    // NOTE: use the order defined in `Kind::iter` so the keys always
    // correspond to the order we display the items in.
    for (index, current_kind) in item::Kind::iter().enumerate() {
        if current_kind == kind {
            return (index + 1) as u8;
        }
    }
    unreachable!()
}

fn kill_monster(monster_position: Point, world: &mut World, audio: &mut Audio) {
    let invincible = world
        .monster_on_pos(monster_position)
        .is_some_and(|m| m.invincible);
    if invincible {
        // It's invincible: no-op
    } else {
        if let Some(monster) = world.monster_on_pos(monster_position) {
            log::debug!("Killing monster: {:?}", monster);
            monster.dead = true;
            audio.mix_sound_effect(Effect::MonsterHit, Duration::from_millis(0));
        }
        world.remove_monster(monster_position);
    }
}

fn use_dose(
    player: &mut player::Player,
    explosion_animation: &mut Option<Box<dyn AreaOfEffect>>,
    item: item::Item,
    palette: &Palette,
    audio: &mut Audio,
) {
    use crate::{item::Kind::*, player::Modifier::*};
    log::debug!("Using dose");
    audio.mix_sound_effect(Effect::Explosion, Duration::from_millis(0));
    // TODO: do a different explosion animation for the cardinal dose
    if let Intoxication { state_of_mind, .. } = item.modifier {
        let radius = if state_of_mind <= 100 { 4 } else { 6 };
        player.take_effect(item.modifier);
        let animation: Box<dyn AreaOfEffect> = match item.kind {
            Dose | StrongDose => Box::new(animation::SquareExplosion::new(
                player.pos,
                radius,
                2,
                palette.explosion,
            )),
            CardinalDose => Box::new(animation::CardinalExplosion::new(
                player.pos,
                radius,
                2,
                palette.explosion,
                palette.shattering_explosion,
            )),
            DiagonalDose => Box::new(animation::DiagonalExplosion::new(
                player.pos,
                radius,
                2,
                palette.explosion,
                palette.shattering_explosion,
            )),
            Food => unreachable!(),
        };
        *explosion_animation = Some(animation);
    } else {
        unreachable!();
    }
}

fn show_exit_stats(stats: &Stats) {
    log::info!(
        "\nSlowest update durations: {:?}\n",
        stats
            .longest_update_durations()
            .iter()
            .map(|dur| dur.as_secs_f32() * 1000.0) // milliseconds in f32
            .rev()
            .collect::<Vec<_>>(),
    );

    log::info!(
        "\nSlowest drawcall durations: {:?}\n",
        stats
            .longest_drawcall_durations()
            .iter()
            .map(|dur| dur.as_secs_f32() * 1000.0) // milliseconds in f32
            .rev()
            .collect::<Vec<_>>(),
    );

    log::info!(
        "\nMean update duration: {} ms\nMean drawcall duration: {} ms",
        stats.mean_update(),
        stats.mean_drawcalls()
    );

    log::info!("Mean FPS: {}", stats.mean_fps());
    log::info!("Lowest FPS: {}", stats.lowest_fps());
}

fn verify_states(expected: &state::Verification, actual: &state::Verification) {
    let mut valid = true;

    if expected.tick_id != actual.tick_id {
        log::error!(
            "Expected tick_id: {}, actual: {}",
            expected.tick_id,
            actual.tick_id
        );
        valid = false;
    }

    if expected.turn != actual.turn {
        log::error!("Expected turn: {}, actual: {}", expected.turn, actual.turn);
        valid = false;
    }

    if expected.chunk_count != actual.chunk_count {
        log::error!(
            "Expected chunks: {}, actual: {}",
            expected.chunk_count,
            actual.chunk_count
        );
        valid = false;
    }

    if expected.player_pos != actual.player_pos {
        log::error!(
            "Expected player position: {}, actual: {}",
            expected.player_pos,
            actual.player_pos
        );
        valid = false;
    }

    if expected.monsters.len() != actual.monsters.len() {
        log::error!(
            "Expected monster count: {}, actual: {}",
            expected.monsters.len(),
            actual.monsters.len()
        );
        valid = false;
    }

    if expected.monsters != actual.monsters {
        let expected_monsters: HashMap<Point, (Point, monster::Kind)> = expected
            .monsters
            .iter()
            .map(|&(pos, chunk_pos, monster)| (pos, (chunk_pos, monster)))
            .collect();
        let actual_monsters: HashMap<Point, (Point, monster::Kind)> = actual
            .monsters
            .iter()
            .map(|&(pos, chunk_pos, monster)| (pos, (chunk_pos, monster)))
            .collect();

        for (pos, expected) in &expected_monsters {
            match actual_monsters.get(pos) {
                Some(actual) => {
                    if expected != actual {
                        log::error!(
                            "Monster at {} differ. Expected: {:?}, \
                             actual: {:?}",
                            pos,
                            expected,
                            actual
                        );
                        valid = false;
                    }
                }
                None => {
                    log::error!(
                        "Monster expected at {}: {:?}, but it's not \
                         there.",
                        pos,
                        expected
                    );
                    valid = false;
                }
            }
        }

        for (pos, actual) in &actual_monsters {
            if !expected_monsters.contains_key(pos) {
                log::error!("There is an unexpected monster at: {}: {:?}.", pos, actual);
                valid = false;
            }
        }
    }

    // Use if+panic rather than `assert_eq`. The latter prints out both objects and they can be MASSIVE.
    // We've got a more targeted expected/actual printout above so all we need here is to crash.
    #[allow(clippy::panic)]
    if !valid {
        panic!("Validation failed!");
    }
}

pub fn create_new_game_state(state: &State, new_challenge: Challenge) -> State {
    let mut state = State::new_game(
        state.world_size,
        state.map_size,
        state.panel_width,
        state.exit_after,
        state.debug,
        state::generate_replay_path(),
        new_challenge,
        state.palette,
    );
    state.generate_world();
    state
}

fn place_victory_npc(state: &mut State) -> Point {
    log::info!("Generating the Victory NPC!");
    let mut distance_range = formula::VICTORY_NPC_DISTANCE;
    // NOTE: Compute path to Victory NPC that is reachable by the
    // player. This may take several attempts. Leave the position
    // immutable at the end.
    let mut vnpc_pos;
    let mut attempts = 250;
    let blockers = Blocker::WALL | Blocker::MONSTER;
    loop {
        if attempts <= 0 {
            // TODO: generate VNPC at a shorter distance instead of crashing?
            log::warn!("Could not find a viable Victory NPC position in 250 tries.");
            let min = distance_range.0 - 20;
            let max = distance_range.1 - 20;
            if min > 5 && max > 5 {
                distance_range = InclusiveRange(min, max);
                attempts = 20;
                log::info!("Reduced VNPC spawn range to: {:?}.", distance_range);
            } else {
                log::warn!(
                    "Could not find a viable Victory NPC position anywhere! Winning game instead."
                );
                state.player.bonuses.push(monster::CompanionBonus::Victory);
                win_the_game(state);
                return state.player.pos;
            }
        } else {
            attempts -= 1;
        }

        // NOTE: this is a little convoluted. We test if the Victory
        // NPC position is walkable. And if it's not, we try other
        // positions in its immediate vicinity instead of generating a
        // new candidate position via `formula::victory_npc_position`.
        //
        // We do this, because the walkability test requires we have a
        // World Chunk in place and generating these can be expensive.
        // So if we picked a completely random position every time, we
        // could end up generating a lot of chunks for no immediate
        // reason.
        //
        // What we do instead is generate one Chunk for the given
        // position and then try nearby areas (which will be
        // overwhelmingly likely in the same Chunk).
        vnpc_pos = formula::victory_npc_position(&mut state.rng, state.player.pos, distance_range);
        log::info!("Trying to find test NPC position {:?}", vnpc_pos);
        state.world.ensure_chunk_at_pos(vnpc_pos);
        if let Some(pos) = walkable_place_nearby(&state.world, vnpc_pos, blockers, state.player.pos)
        {
            log::info!("Position {:?} is walkable!", pos);
            vnpc_pos = pos;
            for cell_pos in point::Line::new(state.player.pos, vnpc_pos) {
                state.world.ensure_chunk_at_pos(cell_pos);
            }
        } else {
            log::warn!(
                "Failed to find empty place around the candidate VNPC position {:?}",
                vnpc_pos
            );
            continue;
        }

        log::info!(
            "player pos: {:?}, vnpc pos: {:?}",
            state.player.pos,
            vnpc_pos
        );
        // TODO: make sure the world chunks exist before trying to find path
        let check_irresistible = false;
        let path_to_vnpc = pathfinding::Path::find(
            state.player.pos,
            vnpc_pos,
            &state.world,
            blockers,
            state.player.pos,
            state.player.will.to_int(),
            check_irresistible,
            formula::PATHFINDING_VNPC_REACHABILITY_LIMIT,
            &pathfinding::direct_cost,
        );
        if path_to_vnpc.is_empty() {
            log::warn!("Failed to find path from player to Victory NPC!")
        } else {
            log::info!("Path to Victory NPC takes {} steps", path_to_vnpc.len());
            break;
        }
    }
    let vnpc_pos = vnpc_pos;

    if let Some(prev_npc_id) = state.victory_npc_id.take() {
        log::warn!("Replacing an existing NPC! {:?}", prev_npc_id);
        state.world.remove_monster_by_id(prev_npc_id);
    }

    // NOTE: Uncover the map leading to the Victory NPC position
    let positions = point::Line::new(state.player.pos, vnpc_pos);
    for cell_pos in positions {
        state.world.ensure_chunk_at_pos(cell_pos);
        let display_half_size = state.map_size / 2;
        // NOTE: make sure every cell that will be shown has a chunk.
        //
        // If we didn't do this, we would get blank places when the line would cross a boundary
        // of two chunks, but the surrounding chunks were not brought in.
        state
            .world
            .ensure_chunk_at_pos(cell_pos + (display_half_size.x, display_half_size.y));
        state
            .world
            .ensure_chunk_at_pos(cell_pos + (-display_half_size.x, display_half_size.y));
        state
            .world
            .ensure_chunk_at_pos(cell_pos + (display_half_size.x, -display_half_size.y));
        state
            .world
            .ensure_chunk_at_pos(cell_pos + (-display_half_size.x, -display_half_size.y));
        state.world.always_visible(cell_pos, 2);
        state.world.explore(cell_pos, 4);
    }
    state.world.explore(vnpc_pos, 5);
    state.world.always_visible(vnpc_pos, 2);

    if let Some(chunk) = state.world.chunk_mut(vnpc_pos) {
        let mut monster = monster::Monster::new(monster::Kind::Npc, vnpc_pos, state.challenge);
        monster.companion_bonus = Some(CompanionBonus::Victory);
        // NOTE: The NPCs have the same colour range as the player,
        // but let's always pick a colour that's different from the
        // current player's one.
        monster.npc_color_index = {
            let mut index: usize = 0;
            for _ in 0..5 {
                let pick = state
                    .rng
                    .range_inclusive(0, state.palette.player.len() as i32 - 1)
                    as usize;
                if pick != state.player.color_index {
                    index = pick;
                    break;
                }
            }
            index
        };

        monster.ai_state = ai::AIState::NoOp;
        let id = chunk.add_monster(monster);
        state.victory_npc_id = Some(id);
    }

    // NOTE: Scroll to the Victory NPC position
    let ms = if state.replay_full_speed { 1000 } else { 2000 };
    state.pos_timer = Timer::new(Duration::from_millis(ms));
    state.old_screen_pos = state.screen_position_in_world;
    state.new_screen_pos = vnpc_pos;

    vnpc_pos
}

fn win_the_game(state: &mut State) {
    state.side = Side::Victory;
    state.game_session = GameSession::Ended;
    state.uncovered_map = true;
    state.selected_endgame_window_action = Some(endgame::Action::NewGame);
    state.window_stack.push(Window::Endgame);
}

/// Return a point close to the given one that is walkable.
fn walkable_place_nearby(
    world: &World,
    pos: Point,
    blockers: Blocker,
    player_pos: Point,
) -> Option<Point> {
    // Radius `2` means the central point and the eight surrounding ones.
    point::SquareArea::new(pos, 2).find(|&point| world.walkable(point, blockers, player_pos))
}
