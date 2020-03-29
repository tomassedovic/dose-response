use crate::color;
use crate::engine::{Display, TextMetrics, TextOptions};
use crate::point::Point;
use crate::rect::Rectangle;
use crate::state::State;
use crate::ui::{self, Text};

#[derive(Debug)]
pub enum MenuItem {
    Resume,
    NewGame,
    Help,
    Settings,
    SaveAndQuit,
    Load,
    Quit,
}

impl MenuItem {
    pub fn to_str(&self) -> &'static str {
        use self::MenuItem::*;
        match self {
            Resume => "[R]esume",
            NewGame => "[N]ew Game",
            Help => "[H]elp",
            Settings => "S[e]ttings",
            SaveAndQuit => "[S]ave and Quit",
            Load => "[L]oad game",
            Quit => "[Q]uit without saving",
        }
    }
}

pub struct Layout<'a> {
    window_rect: Rectangle,
    inner_window_rect: Rectangle,
    rect: Rectangle,
    text_flow: Vec<Text<'a>>,
    menu_item_under_mouse: Option<MenuItem>,
    menu_rect_under_mouse: Option<Rectangle>,
}

pub struct Window;

impl Window {
    fn calculate_layout(
        &self,
        state: &State,
        metrics: &dyn TextMetrics,
        display: &Display,
    ) -> Layout<'_> {
        use crate::ui::Text::*;
        let short = display.size_without_padding().y < 26;

        let window_pos = Point::new(0, 0);
        let window_size = display.size_without_padding();

        let window_rect = Rectangle::from_point_and_size(window_pos, window_size);

        let inner_window_rect = Rectangle::new(
            window_rect.top_left() + (1, 1),
            window_rect.bottom_right() - (1, 1),
        );

        let rect_size = Point::new(20, 15);
        let rect_pos = Point::new((inner_window_rect.width() - rect_size.x) / 2, 0);
        let rect = Rectangle::from_point_and_size(rect_pos, rect_size);

        let title_padding = if short { 1 } else { 2 };
        let mut text_flow = vec![
            EmptySpace(title_padding),
            Centered("Dose Response"),
            Centered("By Tomas Sedovic"),
            EmptySpace(title_padding),
        ];

        let mut options = vec![];

        if !state.game_ended && !state.first_game_already_generated {
            options.push(MenuItem::Resume);
        }

        options.push(MenuItem::NewGame);

        // NOTE: we won't hiding this option, because it would require
        // checking if the file exists every frame (or do some complex
        // caching).
        options.push(MenuItem::Load);

        options.push(MenuItem::Help);

        options.push(MenuItem::Settings);

        if !state.game_ended {
            options.push(MenuItem::SaveAndQuit);
        }
        options.push(MenuItem::Quit);

        let header_rect = ui::text_flow_rect(&text_flow, rect, metrics);

        let mut menu_item_under_mouse = None;
        let mut menu_rect_under_mouse = None;
        let mut ypos = header_rect.bottom_right().y;
        for option in options {
            let text = Centered(option.to_str());
            let text_rect = ui::text_rect(
                &text,
                Rectangle::new(rect.top_left() + (0, ypos), rect.bottom_right()),
                metrics,
            );
            ypos += text_rect.size().y;
            if text_rect.contains(state.mouse.tile_pos) {
                menu_item_under_mouse = Some(option);
                menu_rect_under_mouse = Some(text_rect);
            }
            text_flow.push(text);
            text_flow.push(Empty);
            ypos += ui::text_rect(
                &Empty,
                Rectangle::new(rect.top_left() + (0, ypos), rect.bottom_right()),
                metrics,
            )
            .size()
            .y;
        }

        if window_rect.height() >= 19 {
            let quote_padding = if short { 0 } else { 3 };
            text_flow.push(EmptySpace(quote_padding));
            text_flow.push(Paragraph("\"You cannot lose if you do not play.\""));
            text_flow.push(Paragraph("-- Marla Daniels"));
        }

        Layout {
            window_rect,
            inner_window_rect,
            rect,
            text_flow,
            menu_item_under_mouse,
            menu_rect_under_mouse,
        }
    }

    pub fn render(&self, state: &State, metrics: &dyn TextMetrics, display: &mut Display) {
        let layout = self.calculate_layout(state, metrics, display);
        display.draw_rectangle(layout.window_rect, color::window_edge);
        display.draw_rectangle(layout.inner_window_rect, color::window_background);

        let rect = layout.rect;

        if let Some(rect) = layout.menu_rect_under_mouse {
            display.draw_rectangle(rect, color::menu_highlight);
        }

        ui::render_text_flow(&layout.text_flow, rect, 0, metrics, display);

        // NOTE: draw the version explicitly
        let short = display.size_without_padding().y < 26;
        let version_padding = if short { (0, 0) } else { (1, 1) };
        display.draw_text(
            layout.inner_window_rect.bottom_right() - version_padding,
            &format!(
                "Version: {}.{}",
                crate::metadata::VERSION_MAJOR,
                crate::metadata::VERSION_MINOR
            ),
            color::gui_text,
            TextOptions::align_right(),
        );
    }

    pub fn hovered(
        &self,
        state: &State,
        metrics: &dyn TextMetrics,
        display: &Display,
    ) -> Option<MenuItem> {
        self.calculate_layout(state, metrics, display)
            .menu_item_under_mouse
    }
}
