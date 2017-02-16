use color::Color;
use point::{Point, SquareArea};
use timer::Timer;

use time::Duration;

pub trait AreaOfEffect {
    fn update(&mut self, dt: Duration);
    fn finished(&self) -> bool;
    fn tiles(&self) -> Box<Iterator<Item=(Point, Color, TileEffect)>>;
}

bitflags! {
    pub flags TileEffect: u32 {
        const KILL =    1,
        const SHATTER = 2,
    }
}


#[derive(Debug)]
pub struct SquareExplosion {
    pub center: Point,
    pub max_radius: i32,
    pub initial_radius: i32,
    pub current_radius: i32,
    pub color: Color,
    pub wave_count: i32,
    pub timer: Timer,
}

impl SquareExplosion {
    pub fn new(center: Point, max_radius: i32, initial_radius: i32, color: Color) -> Self {
        assert!(initial_radius <= max_radius);
        // Count the initial wave plus the rest that makes the difference
        let wave_count = max_radius - initial_radius + 1;
        let wave_duration = Duration::milliseconds(100);
        SquareExplosion {
            center: center,
            max_radius: max_radius,
            initial_radius: initial_radius,
            current_radius: initial_radius,
            color: color,
            wave_count: wave_count,
            timer: Timer::new(wave_duration * wave_count),
        }
    }

}

impl AreaOfEffect for SquareExplosion {
    fn update(&mut self, dt: Duration) {
        if self.timer.finished() {
            // do nothing
        } else {
            self.timer.update(dt);
            let single_wave_percentage = 1.0 / (self.wave_count as f32);
            self.current_radius = self.initial_radius + (self.timer.percentage_elapsed() / single_wave_percentage) as i32;
            if self.current_radius > self.max_radius {
                self.current_radius = self.max_radius;
            }
        }
    }

    fn finished(&self) -> bool {
        self.timer.finished()
    }

    fn tiles(&self) -> Box<Iterator<Item=(Point, Color, TileEffect)>> {
        let color = self.color;
        Box::new(
            SquareArea::new(self.center, self.current_radius)
                .map(move |pos| (pos, color, KILL)))
    }

}


#[derive(Debug)]
pub struct CardinalExplosion {
    center: Point,
    max_radius: i32,
    initial_radius: i32,
    current_radius: i32,
    kill_color: Color,
    shatter_color: Color,
    wave_count: i32,
    timer: Timer,
}

impl CardinalExplosion {
    pub fn new(center: Point, max_radius: i32, initial_radius: i32,
               kill_color: Color, shatter_color: Color) -> Self {
        assert!(initial_radius <= max_radius);
        // Count the initial wave plus the rest that makes the difference
        let wave_count = max_radius - initial_radius + 1;
        let wave_duration = Duration::milliseconds(100);
        CardinalExplosion {
            center: center,
            max_radius: max_radius,
            initial_radius: initial_radius,
            current_radius: initial_radius,
            kill_color: kill_color,
            shatter_color: shatter_color,
            wave_count: wave_count,
            timer: Timer::new(wave_duration * wave_count),
        }
    }
}

impl AreaOfEffect for CardinalExplosion {
    fn update(&mut self, dt: Duration) {
        if self.timer.finished() {
            // do nothing
        } else {
            self.timer.update(dt);
            let single_wave_percentage = 1.0 / (self.wave_count as f32);
            self.current_radius = self.initial_radius + (self.timer.percentage_elapsed() / single_wave_percentage) as i32;
            if self.current_radius > self.max_radius {
                self.current_radius = self.max_radius;
            }
        }
    }

    fn finished(&self) -> bool {
        self.timer.finished()
    }

    fn tiles(&self) -> Box<Iterator<Item=(Point, Color, TileEffect)>> {
        let kill_color = self.kill_color;
        let killzone_area = SquareArea::new(self.center, 1)
            .map(move |pos| (pos, kill_color, KILL));

        let shatter_color = self.shatter_color;
        let shatter_area = CrossIterator::new(self.center, self.current_radius)
            .map(move |pos| (pos, shatter_color, KILL & SHATTER));
        Box::new(killzone_area.chain(shatter_area))
    }

}


#[derive(Debug)]
pub struct CrossIterator {
    center: Point,
    range: i32,
    x_offset: i32,
    y_offset: i32,
    horizontal: bool,
    vertical: bool,
}

impl CrossIterator {
    pub fn new(center: Point, range: i32) -> Self {
        assert!(range >= 0);
        CrossIterator {
            center: center,
            range: range,
            x_offset: -range,
            y_offset: -range,
            horizontal: true,
            vertical: false,
        }
    }
}

impl Iterator for CrossIterator {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        if self.horizontal {
            let x_offset = self.x_offset;
            if x_offset <= self.range {
                self.x_offset += 1;
                return Some(self.center + (x_offset, 0));
            } else {
                self.horizontal = false;
                self.vertical = true;
            }
        }

        if self.vertical {
            let y_offset = self.y_offset;
            if y_offset <= self.range {
                self.y_offset += 1;
                return Some(self.center + (0, y_offset));
            } else {
                self.vertical = false;
            }
        }

        None
    }
}



#[derive(Debug)]
pub struct DiagonalExplosion {
    center: Point,
    max_radius: i32,
    initial_radius: i32,
    current_radius: i32,
    kill_color: Color,
    shatter_color: Color,
    wave_count: i32,
    timer: Timer,
}

impl DiagonalExplosion {
    pub fn new(center: Point, max_radius: i32, initial_radius: i32,
               kill_color: Color, shatter_color: Color) -> Self {
        assert!(initial_radius <= max_radius);
        // Count the initial wave plus the rest that makes the difference
        let wave_count = max_radius - initial_radius + 1;
        let wave_duration = Duration::milliseconds(100);
        DiagonalExplosion {
            center: center,
            max_radius: max_radius,
            initial_radius: initial_radius,
            current_radius: initial_radius,
            kill_color: kill_color,
            shatter_color: shatter_color,
            wave_count: wave_count,
            timer: Timer::new(wave_duration * wave_count),
        }
    }
}

impl AreaOfEffect for DiagonalExplosion {
    fn update(&mut self, dt: Duration) {
        if self.timer.finished() {
            // do nothing
        } else {
            self.timer.update(dt);
            let single_wave_percentage = 1.0 / (self.wave_count as f32);
            self.current_radius = self.initial_radius + (self.timer.percentage_elapsed() / single_wave_percentage) as i32;
            if self.current_radius > self.max_radius {
                self.current_radius = self.max_radius;
            }
        }
    }

    fn finished(&self) -> bool {
        self.timer.finished()
    }

    fn tiles(&self) -> Box<Iterator<Item=(Point, Color, TileEffect)>> {
        let kill_color = self.kill_color;
        let killzone_area = SquareArea::new(self.center, 1)
            .map(move |pos| (pos, kill_color, KILL));

        let shatter_color = self.shatter_color;
        let shatter_area = XIterator::new(self.center, self.current_radius)
            .map(move |pos| (pos, shatter_color, KILL & SHATTER));
        Box::new(killzone_area.chain(shatter_area))
    }

}


#[derive(Debug)]
pub struct XIterator {
    center: Point,
    range: i32,
    x_offset: i32,
    y_offset: i32,
    horizontal: bool,
    vertical: bool,
}

impl XIterator {
    pub fn new(center: Point, range: i32) -> Self {
        assert!(range >= 0);
        XIterator {
            center: center,
            range: range,
            x_offset: -range,
            y_offset: -range,
            horizontal: true,
            vertical: false,
        }
    }
}

impl Iterator for XIterator {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        // TODO: simplify the code. This is basically just copied from
        // CrossIterator and the names could be better plus I don't
        // think we need all the fields.
        if self.horizontal {
            let x_offset = self.x_offset;
            if x_offset <= self.range {
                self.x_offset += 1;
                return Some(self.center + (x_offset, -x_offset));
            } else {
                self.horizontal = false;
                self.vertical = true;
            }
        }

        if self.vertical {
            let y_offset = self.y_offset;
            if y_offset <= self.range {
                self.y_offset += 1;
                return Some(self.center + (y_offset, y_offset));
            } else {
                self.vertical = false;
            }
        }

        None
    }
}


#[derive(Debug)]
pub struct ScreenFade {
    pub color: Color,
    pub fade_out_time: Duration,
    pub wait_time: Duration,
    pub fade_in_time: Duration,
    pub timer: Timer,
    pub phase: ScreenFadePhase,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ScreenFadePhase {
    FadeOut,
    Wait,
    FadeIn,
    Done,
}

impl ScreenFade {
    pub fn new(color: Color, fade_out: Duration, wait: Duration, fade_in: Duration) -> Self {
        ScreenFade {
            color: color,
            fade_out_time: fade_out,
            wait_time: wait,
            fade_in_time: fade_in,
            timer: Timer::new(fade_out),
            phase: ScreenFadePhase::FadeOut,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.timer.update(dt);
        if self.timer.finished() {
            match self.phase {
                ScreenFadePhase::FadeOut => {
                    self.timer = Timer::new(self.wait_time);
                    self.phase = ScreenFadePhase::Wait;
                }
                ScreenFadePhase::Wait => {
                    self.timer = Timer::new(self.fade_in_time);
                    self.phase = ScreenFadePhase::FadeIn;
                }
                ScreenFadePhase::FadeIn => {
                    self.phase = ScreenFadePhase::Done;
                }
                ScreenFadePhase::Done => {
                    // NOTE: we're done. Nothing to do here.
                }
            }
        }
    }
}
