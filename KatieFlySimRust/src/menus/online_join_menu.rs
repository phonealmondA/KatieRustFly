// Online Join Menu - Connect to a multiplayer host

use macroquad::prelude::*;
use crate::ui::Button;

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputField {
    IpAddress,
    Port,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnlineJoinMenuResult {
    None,
    Connect(String, u16), // Connect to IP:Port
    Back,
}

pub struct OnlineJoinMenu {
    join_button: Button,
    back_button: Button,
    ip_input: String,
    port_input: String,
    active_field: InputField,
    error_message: Option<String>,
}

impl OnlineJoinMenu {
    pub fn new(window_size: Vec2) -> Self {
        let button_width = 300.0;
        let button_height = 50.0;
        let center_x = window_size.x / 2.0 - button_width / 2.0;
        let start_y = window_size.y / 2.0 + 100.0;
        let spacing = 70.0;

        Self {
            join_button: Button::new(
                Vec2::new(center_x, start_y),
                Vec2::new(button_width, button_height),
                "Connect",
                Color::from_rgba(50, 150, 100, 255),
            ),
            back_button: Button::new(
                Vec2::new(center_x, start_y + spacing),
                Vec2::new(button_width, button_height),
                "Back",
                Color::from_rgba(120, 50, 50, 255),
            ),
            ip_input: "127.0.0.1".to_string(), // Default localhost
            port_input: "7777".to_string(), // Default port
            active_field: InputField::IpAddress,
            error_message: None,
        }
    }

    pub fn update(&mut self) -> OnlineJoinMenuResult {
        let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);

        // Tab to switch fields
        if is_key_pressed(KeyCode::Tab) {
            self.active_field = match self.active_field {
                InputField::IpAddress => InputField::Port,
                InputField::Port => InputField::IpAddress,
            };
        }

        // Handle text input
        if let Some(key) = get_last_key_pressed() {
            let input = match self.active_field {
                InputField::IpAddress => &mut self.ip_input,
                InputField::Port => &mut self.port_input,
            };

            match key {
                KeyCode::Key0 | KeyCode::Kp0 => input.push('0'),
                KeyCode::Key1 | KeyCode::Kp1 => input.push('1'),
                KeyCode::Key2 | KeyCode::Kp2 => input.push('2'),
                KeyCode::Key3 | KeyCode::Kp3 => input.push('3'),
                KeyCode::Key4 | KeyCode::Kp4 => input.push('4'),
                KeyCode::Key5 | KeyCode::Kp5 => input.push('5'),
                KeyCode::Key6 | KeyCode::Kp6 => input.push('6'),
                KeyCode::Key7 | KeyCode::Kp7 => input.push('7'),
                KeyCode::Key8 | KeyCode::Kp8 => input.push('8'),
                KeyCode::Key9 | KeyCode::Kp9 => input.push('9'),
                KeyCode::Period => {
                    // Only allow period in IP address field
                    if matches!(self.active_field, InputField::IpAddress) {
                        input.push('.');
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            }

            // Limit input lengths
            match self.active_field {
                InputField::IpAddress => {
                    if self.ip_input.len() > 15 {
                        self.ip_input.truncate(15);
                    }
                }
                InputField::Port => {
                    if self.port_input.len() > 5 {
                        self.port_input.truncate(5);
                    }
                }
            }
        }

        // Check for button clicks
        if self.join_button.update(mouse_pressed) {
            // Validate IP and port
            match self.port_input.parse::<u16>() {
                Ok(port) if port > 0 => {
                    // Basic IP validation
                    if self.validate_ip(&self.ip_input) {
                        log::info!("Connecting to {}:{}", self.ip_input, port);
                        return OnlineJoinMenuResult::Connect(self.ip_input.clone(), port);
                    } else {
                        self.error_message = Some("Invalid IP address".to_string());
                    }
                }
                _ => {
                    self.error_message = Some("Invalid port number".to_string());
                }
            }
        }

        if self.back_button.update(mouse_pressed) {
            return OnlineJoinMenuResult::Back;
        }

        OnlineJoinMenuResult::None
    }

    fn validate_ip(&self, ip: &str) -> bool {
        // Basic validation - check for 4 octets
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }

        // Check each octet is a valid number 0-255
        for part in parts {
            match part.parse::<u16>() {
                Ok(num) if num <= 255 => {}
                _ => return false,
            }
        }

        true
    }

    pub fn draw(&self) {
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        // Title
        let title = "Join Multiplayer Game";
        let title_size = 48.0;
        let title_dims = measure_text(title, None, title_size as u16, 1.0);
        let title_x = screen_width() / 2.0 - title_dims.width / 2.0;
        let title_y = screen_height() / 2.0 - 220.0;
        draw_text(title, title_x, title_y, title_size, WHITE);

        // Instructions
        let instructions = "Enter host IP address and port (press TAB to switch fields)";
        let inst_size = 20.0;
        let inst_dims = measure_text(instructions, None, inst_size as u16, 1.0);
        let inst_x = screen_width() / 2.0 - inst_dims.width / 2.0;
        let inst_y = title_y + 60.0;
        draw_text(instructions, inst_x, inst_y, inst_size, LIGHTGRAY);

        let input_width = 350.0;
        let input_height = 50.0;
        let input_x = screen_width() / 2.0 - input_width / 2.0;
        let label_size = 20.0;

        // IP Address input
        let ip_y = screen_height() / 2.0 - 80.0;

        // Highlight active field
        let ip_color = if matches!(self.active_field, InputField::IpAddress) {
            Color::new(0.3, 0.3, 0.5, 1.0)
        } else {
            Color::new(0.2, 0.2, 0.3, 1.0)
        };

        draw_rectangle(input_x, ip_y, input_width, input_height, ip_color);
        draw_rectangle_lines(
            input_x,
            ip_y,
            input_width,
            input_height,
            2.0,
            if matches!(self.active_field, InputField::IpAddress) { YELLOW } else { WHITE }
        );

        // IP label
        let ip_label = "IP Address:";
        let ip_label_dims = measure_text(ip_label, None, label_size as u16, 1.0);
        draw_text(ip_label, input_x - ip_label_dims.width - 20.0, ip_y + 32.0, label_size, WHITE);

        // IP text
        let ip_text_size = 28.0;
        let ip_text_dims = measure_text(&self.ip_input, None, ip_text_size as u16, 1.0);
        let ip_text_x = input_x + input_width / 2.0 - ip_text_dims.width / 2.0;
        draw_text(&self.ip_input, ip_text_x, ip_y + 33.0, ip_text_size, WHITE);

        // IP cursor
        if matches!(self.active_field, InputField::IpAddress) && (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = ip_text_x + ip_text_dims.width + 5.0;
            draw_rectangle(cursor_x, ip_y + 10.0, 2.0, 30.0, YELLOW);
        }

        // Port input
        let port_y = ip_y + 80.0;

        let port_color = if matches!(self.active_field, InputField::Port) {
            Color::new(0.3, 0.3, 0.5, 1.0)
        } else {
            Color::new(0.2, 0.2, 0.3, 1.0)
        };

        draw_rectangle(input_x, port_y, input_width, input_height, port_color);
        draw_rectangle_lines(
            input_x,
            port_y,
            input_width,
            input_height,
            2.0,
            if matches!(self.active_field, InputField::Port) { YELLOW } else { WHITE }
        );

        // Port label
        let port_label = "Port:";
        let port_label_dims = measure_text(port_label, None, label_size as u16, 1.0);
        draw_text(port_label, input_x - port_label_dims.width - 20.0, port_y + 32.0, label_size, WHITE);

        // Port text
        let port_text_size = 28.0;
        let port_text_dims = measure_text(&self.port_input, None, port_text_size as u16, 1.0);
        let port_text_x = input_x + input_width / 2.0 - port_text_dims.width / 2.0;
        draw_text(&self.port_input, port_text_x, port_y + 33.0, port_text_size, WHITE);

        // Port cursor
        if matches!(self.active_field, InputField::Port) && (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = port_text_x + port_text_dims.width + 5.0;
            draw_rectangle(cursor_x, port_y + 10.0, 2.0, 30.0, YELLOW);
        }

        // Draw buttons
        self.join_button.draw();
        self.back_button.draw();

        // Draw error message if any
        if let Some(ref error) = self.error_message {
            let error_size = 24.0;
            let error_dims = measure_text(error, None, error_size as u16, 1.0);
            let error_x = screen_width() / 2.0 - error_dims.width / 2.0;
            let error_y = screen_height() / 2.0 + 240.0;
            draw_text(error, error_x, error_y, error_size, RED);
        }

        // Info text
        let info = "Connect to a friend's hosted game using their IP address";
        let info_size = 18.0;
        let info_dims = measure_text(info, None, info_size as u16, 1.0);
        let info_x = screen_width() / 2.0 - info_dims.width / 2.0;
        let info_y = screen_height() - 80.0;
        draw_text(info, info_x, info_y, info_size, YELLOW);

        let info2 = "For local testing, use 127.0.0.1 (localhost)";
        let info2_dims = measure_text(info2, None, info_size as u16, 1.0);
        let info2_x = screen_width() / 2.0 - info2_dims.width / 2.0;
        draw_text(info2, info2_x, info_y + 25.0, info_size, YELLOW);
    }
}
