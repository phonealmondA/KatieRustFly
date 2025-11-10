// Online Multiplayer Menu - Host or join online games

use macroquad::prelude::*;
use crate::ui::Button;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnlineMultiplayerMenuResult {
    None,
    Host,
    Join,
    Back,
}

/// Online multiplayer menu
pub struct OnlineMultiplayerMenu {
    title: String,
    host_button: Button,
    join_button: Button,
    back_button: Button,
}

impl OnlineMultiplayerMenu {
    pub fn new(window_size: Vec2) -> Self {
        let button_width = 300.0;
        let button_height = 50.0;
        let center_x = window_size.x / 2.0 - button_width / 2.0;
        let start_y = window_size.y / 2.0 - 50.0;
        let spacing = 70.0;

        OnlineMultiplayerMenu {
            title: "Online Multiplayer".to_string(),
            host_button: Button::new(
                Vec2::new(center_x, start_y),
                Vec2::new(button_width, button_height),
                "Host Game",
                Color::from_rgba(50, 100, 150, 255),
            ),
            join_button: Button::new(
                Vec2::new(center_x, start_y + spacing),
                Vec2::new(button_width, button_height),
                "Join Game",
                Color::from_rgba(50, 120, 100, 255),
            ),
            back_button: Button::new(
                Vec2::new(center_x, start_y + spacing * 2.0),
                Vec2::new(button_width, button_height),
                "Back",
                Color::from_rgba(120, 50, 50, 255),
            ),
        }
    }

    /// Update menu and return result
    pub fn update(&mut self) -> OnlineMultiplayerMenuResult {
        let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);

        // Check for button clicks
        if self.host_button.update(mouse_pressed) {
            return OnlineMultiplayerMenuResult::Host;
        }

        if self.join_button.update(mouse_pressed) {
            return OnlineMultiplayerMenuResult::Join;
        }

        if self.back_button.update(mouse_pressed) {
            return OnlineMultiplayerMenuResult::Back;
        }

        OnlineMultiplayerMenuResult::None
    }

    /// Render menu
    pub fn draw(&self) {
        // Clear background
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        // Draw title
        let title_size = 48.0;
        let title_dims = measure_text(&self.title, None, title_size as u16, 1.0);
        let title_x = screen_width() / 2.0 - title_dims.width / 2.0;
        let title_y = screen_height() / 2.0 - 200.0;

        draw_text(&self.title, title_x, title_y, title_size, WHITE);

        // Draw buttons
        self.host_button.draw();
        self.join_button.draw();
        self.back_button.draw();

        // Draw info text
        let info = "Choose to host a game or join an existing one";
        let info_size = 20.0;
        let info_dims = measure_text(info, None, info_size as u16, 1.0);
        let info_x = screen_width() / 2.0 - info_dims.width / 2.0;
        let info_y = screen_height() - 100.0;
        draw_text(info, info_x, info_y, info_size, LIGHTGRAY);

        let note = "UDP-based multiplayer with 10-15 second snapshot sync";
        let note_size = 18.0;
        let note_dims = measure_text(note, None, note_size as u16, 1.0);
        let note_x = screen_width() / 2.0 - note_dims.width / 2.0;
        draw_text(note, note_x, info_y + 30.0, note_size, YELLOW);
    }
}
