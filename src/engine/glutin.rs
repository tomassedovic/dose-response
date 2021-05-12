use crate::{
    color::Color,
    engine::{
        self,
        loop_state::{LoopState, ResizeWindowAction, UpdateResult},
    },
    keys::{Key, KeyCode},
    point::Point,
    settings::Store as SettingsStore,
    state::State,
};

use std::time::Instant;

use glutin::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode as BackendKey, WindowEvent},
    monitor::MonitorHandle,
    window::Fullscreen,
};

fn key_code_from_backend(backend_code: BackendKey) -> Option<KeyCode> {
    match backend_code {
        BackendKey::Return => Some(KeyCode::Enter),
        BackendKey::Escape => Some(KeyCode::Esc),
        BackendKey::Space => Some(KeyCode::Space),

        BackendKey::Key0 => Some(KeyCode::D0),
        BackendKey::Key1 => Some(KeyCode::D1),
        BackendKey::Key2 => Some(KeyCode::D2),
        BackendKey::Key3 => Some(KeyCode::D3),
        BackendKey::Key4 => Some(KeyCode::D4),
        BackendKey::Key5 => Some(KeyCode::D5),
        BackendKey::Key6 => Some(KeyCode::D6),
        BackendKey::Key7 => Some(KeyCode::D7),
        BackendKey::Key8 => Some(KeyCode::D8),
        BackendKey::Key9 => Some(KeyCode::D9),

        BackendKey::A => Some(KeyCode::A),
        BackendKey::B => Some(KeyCode::B),
        BackendKey::C => Some(KeyCode::C),
        BackendKey::D => Some(KeyCode::D),
        BackendKey::E => Some(KeyCode::E),
        BackendKey::F => Some(KeyCode::F),
        BackendKey::G => Some(KeyCode::G),
        BackendKey::H => Some(KeyCode::H),
        BackendKey::I => Some(KeyCode::I),
        BackendKey::J => Some(KeyCode::J),
        BackendKey::K => Some(KeyCode::K),
        BackendKey::L => Some(KeyCode::L),
        BackendKey::M => Some(KeyCode::M),
        BackendKey::N => Some(KeyCode::N),
        BackendKey::O => Some(KeyCode::O),
        BackendKey::P => Some(KeyCode::P),
        BackendKey::Q => Some(KeyCode::Q),
        BackendKey::R => Some(KeyCode::R),
        BackendKey::S => Some(KeyCode::S),
        BackendKey::T => Some(KeyCode::T),
        BackendKey::U => Some(KeyCode::U),
        BackendKey::V => Some(KeyCode::V),
        BackendKey::W => Some(KeyCode::W),
        BackendKey::X => Some(KeyCode::X),
        BackendKey::Y => Some(KeyCode::Y),
        BackendKey::Z => Some(KeyCode::Z),

        BackendKey::F1 => Some(KeyCode::F1),
        BackendKey::F2 => Some(KeyCode::F2),
        BackendKey::F3 => Some(KeyCode::F3),
        BackendKey::F4 => Some(KeyCode::F4),
        BackendKey::F5 => Some(KeyCode::F5),
        BackendKey::F6 => Some(KeyCode::F6),
        BackendKey::F7 => Some(KeyCode::F7),
        BackendKey::F8 => Some(KeyCode::F8),
        BackendKey::F9 => Some(KeyCode::F9),
        BackendKey::F10 => Some(KeyCode::F10),
        BackendKey::F11 => Some(KeyCode::F11),
        BackendKey::F12 => Some(KeyCode::F12),

        BackendKey::Right => Some(KeyCode::Right),
        BackendKey::Left => Some(KeyCode::Left),
        BackendKey::Down => Some(KeyCode::Down),
        BackendKey::Up => Some(KeyCode::Up),

        // NOTE: these keys trigger on the numpad when NumLock is off.
        // We will translate them back to the appropriate numpad keys:
        BackendKey::Home => Some(KeyCode::NumPad7),
        BackendKey::End => Some(KeyCode::NumPad1),
        BackendKey::PageUp => Some(KeyCode::NumPad9),
        BackendKey::PageDown => Some(KeyCode::NumPad3),

        BackendKey::Numpad1 => Some(KeyCode::NumPad1),
        BackendKey::Numpad2 => Some(KeyCode::NumPad2),
        BackendKey::Numpad3 => Some(KeyCode::NumPad3),
        BackendKey::Numpad4 => Some(KeyCode::NumPad4),
        BackendKey::Numpad5 => Some(KeyCode::NumPad5),
        BackendKey::Numpad6 => Some(KeyCode::NumPad6),
        BackendKey::Numpad7 => Some(KeyCode::NumPad7),
        BackendKey::Numpad8 => Some(KeyCode::NumPad8),
        BackendKey::Numpad9 => Some(KeyCode::NumPad9),
        BackendKey::Numpad0 => Some(KeyCode::NumPad0),

        _ => None,
    }
}

fn get_current_monitor(monitors: &[MonitorHandle], window_pos: Point) -> Option<MonitorHandle> {
    for monitor in monitors {
        let monitor_pos = {
            let pos = monitor.position();
            Point::new(pos.x as i32, pos.y as i32)
        };
        let monitor_dimensions = {
            let dim = monitor.size();
            Point::new(dim.width as i32, dim.height as i32)
        };

        let monitor_bottom_left = monitor_pos + monitor_dimensions;
        if window_pos >= monitor_pos && window_pos < monitor_bottom_left {
            return Some(monitor.clone());
        }
    }

    monitors.iter().cloned().next()
}

#[allow(cyclomatic_complexity, unsafe_code)]
pub fn main_loop<S>(
    initial_game_display_size: Point,
    initial_default_background: Color,
    window_title: &str,
    mut settings_store: S,
    initial_state: Box<State>,
) where
    S: SettingsStore + 'static,
{
    // Force winit unix backend to X11.
    //
    // Right now, this produces better results on Wayland (Fedora 28).
    // Ideally, we should remove this once winit looks better. We're
    // using `winit 0.18`, the latest release as of writing this.
    //
    // Here are the current issues under wayland:
    // 1. The window decorations look different from the rest of the system
    // 2. The full screen just maximises the window -- the decorations are still visible.
    //    - NOTE: we can use `window.set_decorations(false)` to fix it
    //    - still, feels like we shouldn't have to
    //
    // Both are fixed with the line below:
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let mut loop_state = LoopState::initialise(
        settings_store.load(),
        initial_game_display_size,
        initial_default_background,
        initial_state,
    );

    let event_loop = glutin::event_loop::EventLoop::new();
    log::debug!("Created events loop: {:?}", event_loop);
    let desired_size = {
        let size = loop_state.desired_window_size_px();
        LogicalSize::new(size.0, size.1)
    };
    let window = glutin::window::WindowBuilder::new()
        .with_title(window_title)
        .with_min_inner_size(LogicalSize::new(480, 320))
        .with_inner_size(desired_size);
    #[cfg(target_os = "windows")]
    let window = {
        use glutin::platform::windows::WindowBuilderExtWindows;
        log::debug!("We're on Windows, disabling drag and drop!");
        window.with_drag_and_drop(false)
    };
    log::debug!("Created window builder: {:?}", window);
    let context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(window, &event_loop);
    log::debug!("Created context: {:?}", context);
    let context = match context {
        Ok(context) => context,
        Err(error) => {
            log::error!("Could not create context: {:?}", error);
            panic!("Aborting!");
        }
    };

    log::debug!("Making context current.");
    let context = unsafe {
        match context.make_current() {
            Ok(context) => context,
            Err(error) => {
                log::error!("Could not make context current: {:?}", error);
                panic!("Aborting!");
            }
        }
    };

    gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);
    log::debug!("Loaded OpenGL symbols.");

    let opengl_app = loop_state.opengl_app();

    let dpi = context.window().scale_factor();
    log::info!("Window HIDPI factor: {:?}", dpi);
    log::info!(
        "Window inner size (physical): {:?}",
        context.window().inner_size()
    );

    log::info!(
        "Window outer size (physical): {:?}",
        context.window().outer_size()
    );

    // We'll just assume the monitors won't change throughout the game.
    let monitors: Vec<_> = event_loop.available_monitors().collect();
    log::debug!("Got all available monitors: {:?}", monitors);

    let mut window_pos = context
        .window()
        .outer_position()
        .map(|p| Point::new(p.x as i32, p.y as i32))
        .unwrap_or_default();
    log::info!("Window pos: {:?}", window_pos);
    let mut pre_fullscreen_window_pos = window_pos;

    let mut current_monitor = get_current_monitor(&monitors, window_pos);
    log::debug!("All monitors:");
    for monitor in &monitors {
        log::debug!(
            "* {:?}, pos: {:?}, size: {:?}",
            monitor.name(),
            monitor.position(),
            monitor.size()
        );
    }
    log::info!(
        "Current monitor: {:?}, pos: {:?}, size: {:?}",
        current_monitor.as_ref().map(|m| m.name()),
        current_monitor.as_ref().map(|m| m.position()),
        current_monitor.as_ref().map(|m| m.size())
    );

    let mut previous_frame_start_time = Instant::now();
    let mut modifiers = Default::default();

    event_loop.run(move |event, _, control_flow| {
        log::debug!("{:?}", event);
        match event {
            Event::NewEvents(..) => {
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = glutin::event_loop::ControlFlow::Exit,

                WindowEvent::Resized(size) => {
                    log::info!("WindowEvent::Resized: {:?}", size);
                    let logical_size: LogicalSize<i32> = size.to_logical(dpi);

                    if let Some(monitor_id) = context.window().fullscreen() {
                        log::warn!(
                            "Asked to resize on fullscreen: target size: {:?}, monitor ID: {:?}. Ignoring this request.",
                            size,
                            monitor_id);
                    }

                    context.resize(size);
                    loop_state.handle_window_size_changed(logical_size.width as i32,
                                                          logical_size.height as i32);
                }

                WindowEvent::Moved(new_pos) => {
                    if loop_state.settings.fullscreen || loop_state.switched_from_fullscreen {
                        // Don't update the window position
                        //
                        // Even after we switch from
                        // fullscreen, the `Moved` event has a
                        // wrong value that messes things up.
                        // So we restore the previous position
                        // manually instead.
                    } else {
                        log::debug!(
                            "[FRAME {}] Window moved to: {:?}",
                            loop_state.current_frame_id,
                            new_pos
                        );
                        window_pos.x = new_pos.x as i32;
                        window_pos.y = new_pos.y as i32;
                        current_monitor = get_current_monitor(&monitors, window_pos);
                        log::debug!(
                            "Current monitor: {:?}, pos: {:?}, size: {:?}",
                            current_monitor.as_ref().map(|m| m.name()),
                            current_monitor.as_ref().map(|m| m.position()),
                            current_monitor.as_ref().map(|m| m.size())
                        );
                    }
                }

                WindowEvent::ModifiersChanged(state) => {
                    log::debug!("Modifiers changed: {:?}", state);
                    modifiers = state;
                }

                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode: Some(backend_code),
                        state: ElementState::Pressed,
                        scancode,
                        ..
                    },
                    ..
                } => {
                    log::debug!(
                        "KeyDown backend_code: {:?}, scancode: {:?}, modifiers: {:?}",
                        backend_code,
                        scancode,
                        modifiers,
                    );
                    if let Some(code) = key_code_from_backend(backend_code) {
                        let key = Key {
                            code,
                            alt: modifiers.alt(),
                            ctrl: modifiers.ctrl(),
                            shift: modifiers.shift(),
                            logo: modifiers.logo(),
                        };
                        log::debug!("Detected key {:?}", key);
                        loop_state.keys.push(key);
                    }
                }

                WindowEvent::ReceivedCharacter(chr) => {
                    log::debug!("Received character: {:?}", chr);
                    if chr == '?' {
                        let key = Key {
                            code: KeyCode::QuestionMark,
                            alt: false,
                            ctrl: false,
                            shift: false,
                            logo: false,
                        };
                        log::debug!("Detected key {:?}", key);
                        loop_state.keys.push(key);
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    // NOTE: This function expects logical, not physical pixels.
                    // But the values we get in this event are physical, so we need
                    // to divide by the DPI to mae them logical.
                    loop_state.update_mouse_position(dpi,
                                                     (position.x / dpi) as i32,
                                                     (position.y / dpi) as i32);
                }

                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button,
                    ..
                } => {
                    use glutin::event::MouseButton::*;
                    match button {
                        Left => {
                            loop_state.mouse.left_is_down = true;
                        }
                        Right => {
                            loop_state.mouse.right_is_down = true;
                        }
                        _ => {}
                    }
                }

                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button,
                    ..
                } => {
                    use glutin::event::MouseButton::*;
                    match button {
                        Left => {
                            loop_state.mouse.left_clicked = true;
                            loop_state.mouse.left_is_down = false;
                        }
                        Right => {
                            loop_state.mouse.right_clicked = true;
                            loop_state.mouse.right_is_down = false;
                        }
                        _ => {}
                    }
                }

                _ => (),
            }
            Event::MainEventsCleared => {

                let frame_start_time = Instant::now();
                let dt = frame_start_time.duration_since(previous_frame_start_time);
                previous_frame_start_time = frame_start_time;

                loop_state.update_fps(dt);

                match loop_state.update_game(dt, &mut settings_store) {
                    UpdateResult::QuitRequested => *control_flow = glutin::event_loop::ControlFlow::Exit,
                    UpdateResult::KeepGoing => {}
                }



                if cfg!(feature = "fullscreen") {
                    use engine::loop_state::FullscreenAction::*;
                    match loop_state.fullscreen_action() {
                        Some(SwitchToFullscreen) => {
                            if let Some(ref monitor) = current_monitor {
                                pre_fullscreen_window_pos = window_pos;
                                log::info!(
                                    "Monitor: {:?}, pos: {:?}, dimensions: {:?}",
                                    monitor.name(),
                                    monitor.position(),
                                    monitor.size()
                                );
                                // TODO: let's see if we need to set
                                // decorations explicitly. Remove this line if
                                // we don't actually need it.
                                //context.window().set_decorations(false);
                                context.window().set_fullscreen(Some(Fullscreen::Borderless(monitor.clone())));
                            } else {
                                log::warn!("`current_monitor` is not set!??");
                            }
                        }
                        Some(SwitchToWindowed) => {
                            let window = context.window();
                            window.set_fullscreen(None);
                            let pos = window.outer_position();
                            log::info!("New window position: {:?}", pos);
                            window.set_decorations(true);
                            loop_state.switched_from_fullscreen = true;
                        }
                        None => {}
                    };
                }

                match loop_state.check_window_size_needs_updating() {
                    ResizeWindowAction::NewSize(desired_window_size_px) => {
                        log::info!("Updating window to new size: {:?}", desired_window_size_px);
                        let window = context.window();
                        let size: LogicalSize<u32> = desired_window_size_px.into();
                        context.resize(size.to_physical(window.scale_factor()));
                    }
                    ResizeWindowAction::NoChange => {}
                }

                context.window().request_redraw();
            }
            Event::RedrawRequested(_window_id) => {

                loop_state.process_vertices_and_render(&opengl_app, dpi);
                context.swap_buffers().unwrap();

                loop_state.previous_settings = loop_state.settings.clone();

                if loop_state.current_frame_id == 1 {
                    // NOTE: We should have the proper window position and
                    // monitor info at this point but not sooner.

                    // NOTE: If the primary monitor is different from the
                    // monitor the window actually spawns at (this happens on
                    // my dev machine where the primary monitor is in the
                    // portrait orientation and therefore more narrow, but the
                    // game window normally spawns on my landscape monitor),
                    // it gets resized. We can detect it because this event
                    // fires on the first frame. So we ask it to resize to the
                    // expected size again and leave it at that.
                    log::info!(
                        "Current monitor size: {:?}",
                        current_monitor.as_ref().map(|m| m.size())
                    );
                }

                // If we just switched from fullscreen back to a windowed
                // mode, restore the window position we had before. We do this
                // because the `Moved` event fires with an incorrect value
                // when coming back from full screen.
                //
                // This ensures that we can switch full screen back and fort
                // on a multi monitor setup.
                if loop_state.switched_from_fullscreen {
                    window_pos = pre_fullscreen_window_pos;
                }

            }
            Event::LoopDestroyed => {
                log::info!(
                    "Drawcall count: {}. Capacity: {}.",
                    loop_state.overall_max_drawcall_count,
                    engine::DRAWCALL_CAPACITY
                );

            }
            _ => {}
        }
    });
}
