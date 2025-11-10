// Online Host Menu - Configure and start hosting a multiplayer game

use macroquad::prelude::*;
use crate::ui::Button;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnlineHostMenuResult {
    None,
    StartHost(u16), // Start hosting on specified port
    Back,
}

pub struct OnlineHostMenu {
    host_button: Button,
    back_button: Button,
    port_input: String,
    error_message: Option<String>,
}

impl OnlineHostMenu {
    pub fn new(window_size: Vec2) -> Self {
        let button_width = 300.0;
        let button_height = 50.0;
        let center_x = window_size.x / 2.0 - button_width / 2.0;
        let start_y = window_size.y / 2.0 + 50.0;
        let spacing = 70.0;

        Self {
            host_button: Button::new(
                Vec2::new(center_x, start_y),
                Vec2::new(button_width, button_height),
                "Start Hosting",
                Color::from_rgba(50, 150, 100, 255),
            ),
            back_button: Button::new(
                Vec2::new(center_x, start_y + spacing),
                Vec2::new(button_width, button_height),
                "Back",
                Color::from_rgba(120, 50, 50, 255),
            ),
            port_input: "7777".to_string(), // Default port
            error_message: None,
        }
    }

    pub fn update(&mut self) -> OnlineHostMenuResult {
        let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);

        // Handle text input for port
        if let Some(key) = get_last_key_pressed() {
            match key {
                KeyCode::Key0 | KeyCode::Kp0 => self.port_input.push('0'),
                KeyCode::Key1 | KeyCode::Kp1 => self.port_input.push('1'),
                KeyCode::Key2 | KeyCode::Kp2 => self.port_input.push('2'),
                KeyCode::Key3 | KeyCode::Kp3 => self.port_input.push('3'),
                KeyCode::Key4 | KeyCode::Kp4 => self.port_input.push('4'),
                KeyCode::Key5 | KeyCode::Kp5 => self.port_input.push('5'),
                KeyCode::Key6 | KeyCode::Kp6 => self.port_input.push('6'),
                KeyCode::Key7 | KeyCode::Kp7 => self.port_input.push('7'),
                KeyCode::Key8 | KeyCode::Kp8 => self.port_input.push('8'),
                KeyCode::Key9 | KeyCode::Kp9 => self.port_input.push('9'),
                KeyCode::Backspace => {
                    self.port_input.pop();
                }
                _ => {}
            }

            // Limit port length
            if self.port_input.len() > 5 {
                self.port_input.truncate(5);
            }
        }

        // Check for button clicks
        if self.host_button.update(mouse_pressed) {
            // Validate port
            match self.port_input.parse::<u16>() {
                Ok(port) if port > 0 => {
                    log::info!("Starting host on port {}", port);
                    return OnlineHostMenuResult::StartHost(port);
                }
                _ => {
                    self.error_message = Some("Invalid port number".to_string());
                }
            }
        }

        if self.back_button.update(mouse_pressed) {
            return OnlineHostMenuResult::Back;
        }

        OnlineHostMenuResult::None
    }

    pub fn draw(&self) {
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        // Title
        let title = "Host Multiplayer Game";
        let title_size = 48.0;
        let title_dims = measure_text(title, None, title_size as u16, 1.0);
        let title_x = screen_width() / 2.0 - title_dims.width / 2.0;
        let title_y = screen_height() / 2.0 - 200.0;
        draw_text(title, title_x, title_y, title_size, WHITE);

        // Instructions
        let instructions = "Enter port number and click 'Start Hosting'";
        let inst_size = 20.0;
        let inst_dims = measure_text(instructions, None, inst_size as u16, 1.0);
        let inst_x = screen_width() / 2.0 - inst_dims.width / 2.0;
        let inst_y = title_y + 60.0;
        draw_text(instructions, inst_x, inst_y, inst_size, LIGHTGRAY);

        // Port input box
        let input_y = screen_height() / 2.0 - 30.0;
        let input_width = 300.0;
        let input_height = 50.0;
        let input_x = screen_width() / 2.0 - input_width / 2.0;

        // Draw input box background
        draw_rectangle(input_x, input_y, input_width, input_height, Color::new(0.2, 0.2, 0.3, 1.0));
        draw_rectangle_lines(input_x, input_y, input_width, input_height, 2.0, WHITE);

        // Draw label
        let label = "Port:";
        let label_size = 20.0;
        let label_dims = measure_text(label, None, label_size as u16, 1.0);
        draw_text(label, input_x - label_dims.width - 20.0, input_y + 32.0, label_size, WHITE);

        // Draw port text
        let port_text_size = 30.0;
        let port_dims = measure_text(&self.port_input, None, port_text_size as u16, 1.0);
        let port_x = input_x + input_width / 2.0 - port_dims.width / 2.0;
        draw_text(&self.port_input, port_x, input_y + 35.0, port_text_size, WHITE);

        // Draw cursor blink
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = port_x + port_dims.width + 5.0;
            draw_rectangle(cursor_x, input_y + 10.0, 2.0, 30.0, WHITE);
        }

        // Draw buttons
        self.host_button.draw();
        self.back_button.draw();

        // Draw error message if any
        if let Some(ref error) = self.error_message {
            let error_size = 24.0;
            let error_dims = measure_text(error, None, error_size as u16, 1.0);
            let error_x = screen_width() / 2.0 - error_dims.width / 2.0;
            let error_y = screen_height() / 2.0 + 200.0;
            draw_text(error, error_x, error_y, error_size, RED);
        }

        // Info text
        let info = "Other players will need to connect to your IP address";
        let info_size = 18.0;
        let info_dims = measure_text(info, None, info_size as u16, 1.0);
        let info_x = screen_width() / 2.0 - info_dims.width / 2.0;
        let info_y = screen_height() - 80.0;
        draw_text(info, info_x, info_y, info_size, YELLOW);

        let info2 = "Recommended port: 7777 (make sure it's not blocked by firewall)";
        let info2_dims = measure_text(info2, None, info_size as u16, 1.0);
        let info2_x = screen_width() / 2.0 - info2_dims.width / 2.0;
        draw_text(info2, info2_x, info_y + 25.0, info_size, YELLOW);
    }
}
