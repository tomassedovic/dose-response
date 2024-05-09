use gilrs::{Button, Event, Gilrs};

#[derive(Copy, Clone, Debug, Default)]
pub struct Gamepad {
    /// D-Pad Up
    pub up: bool,

    /// D-Pad Down
    pub down: bool,

    /// D-Pad Left
    pub left: bool,

    /// D-Pad Right
    pub right: bool,

    /// Y or Triangle
    pub north: bool,

    /// A or Cross
    pub south: bool,

    /// X or Square
    pub west: bool,

    /// B or Circle
    pub east: bool,

    /// The button next to the D-pad cluster on the left (Share)
    pub select: bool,

    /// The button next to the A/B/X/Y cluster on the right (Options)
    pub start: bool,

    /// Left stick X axis value in range <-1.0, 1.0>
    pub left_stick_x: f32,

    /// Left stick Y axis value in range <-1.0, 1.0>
    pub left_stick_y: f32,

    /// True if the left stick was "flicked" (or "ticked" or whatever)
    /// i.e. moved in a direction in this frame.
    ///
    /// Once that happens, it won't be considered "flicked" again
    /// until it's returned to the neutral position.
    ///
    /// This makes the stick's behaviour treatable like a button press
    /// rather than something always producing values.
    pub left_stick_flicked: bool,

    // TODO: make this private actaully
    pub ready_for_a_flick: bool,
}

impl Gamepad {
    pub fn reset_buttons(&mut self) {
        *self = Gamepad {
            left_stick_x: self.left_stick_x,
            left_stick_y: self.left_stick_y,
            left_stick_flicked: self.left_stick_flicked,
            ready_for_a_flick: self.ready_for_a_flick,
            ..Gamepad::default()
        }
    }
}

pub fn process_gamepad_events(gilrs: &mut Gilrs, gamepad: &mut Gamepad) {
    // TODO: we're going to have to handle button presses and releases I think
    while let Some(Event {
        id: _,
        event,
        time: _,
    }) = gilrs.next_event()
    {
        match event {
            gilrs::EventType::ButtonPressed(button, code) => match button {
                Button::DPadUp => gamepad.up = true,
                Button::DPadDown => gamepad.down = true,
                Button::DPadLeft => gamepad.left = true,
                Button::DPadRight => gamepad.right = true,

                Button::South => gamepad.south = true,
                Button::East => gamepad.east = true,
                Button::North => gamepad.north = true,
                Button::West => gamepad.west = true,

                Button::Start => gamepad.start = true,
                Button::Select => gamepad.select = true,

                _ => {
                    log::info!(
                        "Pressed a gamepad button that wasn't handled: {:?} {:?}",
                        button,
                        code
                    );
                }
            },

            gilrs::EventType::AxisChanged(axis, value, _code) => {
                use gilrs::ev::Axis::*;
                match axis {
                    LeftStickX => {
                        gamepad.left_stick_x = value;
                    }
                    LeftStickY => {
                        gamepad.left_stick_y = value;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let threshold = 0.4;
    if gamepad.left_stick_x > -threshold && gamepad.left_stick_x < threshold {
        gamepad.left_stick_x = 0.0;
    }
    if gamepad.left_stick_y > -threshold && gamepad.left_stick_y < threshold {
        gamepad.left_stick_y = 0.0;
    }

    if !gamepad.ready_for_a_flick && gamepad.left_stick_x == 0.0 && gamepad.left_stick_y == 0.0 {
        gamepad.ready_for_a_flick = true;
    }

    if gamepad.ready_for_a_flick && (gamepad.left_stick_x != 0.0 || gamepad.left_stick_y != 0.0) {
        gamepad.ready_for_a_flick = false;
        gamepad.left_stick_flicked = true;
    } else {
        gamepad.left_stick_flicked = false;
    }
}
