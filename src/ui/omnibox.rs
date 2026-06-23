use std::collections::VecDeque;

pub struct OmniboxState {
    pub is_focused: bool,
    pub input: String,
    pub cursor_position: usize,
    pub select_all_on_type: bool,
    pub history: VecDeque<String>,
    pub history_index: Option<usize>,
}

impl OmniboxState {
    pub fn new() -> Self {
        Self {
            is_focused: false,
            input: String::new(),
            cursor_position: 0,
            select_all_on_type: false,
            history: VecDeque::with_capacity(16),
            history_index: None,
        }
    }

    pub fn focus(&mut self, current_url: &str) {
        self.is_focused = true;
        self.input = current_url.to_string();
        self.cursor_position = self.input.len();
        self.select_all_on_type = true;
        self.history_index = None;
    }

    pub fn defocus(&mut self) {
        self.is_focused = false;
        self.select_all_on_type = false;
        self.history_index = None;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.select_all_on_type {
            self.input.clear();
            self.cursor_position = 0;
            self.select_all_on_type = false;
        }
        if self.cursor_position <= self.input.len() {
            self.input.insert(self.cursor_position, c);
            self.cursor_position += c.len_utf8();
        }
    }

    pub fn backspace(&mut self) {
        if self.select_all_on_type {
            self.input.clear();
            self.cursor_position = 0;
            self.select_all_on_type = false;
            return;
        }
        if self.cursor_position > 0 {
            // Remove the last char before cursor by checking char boundaries
            let prev_char_idx = self.input[..self.cursor_position].chars().last().map(|c| self.cursor_position - c.len_utf8());
            if let Some(idx) = prev_char_idx {
                self.input.remove(idx);
                self.cursor_position = idx;
            }
        }
    }

    pub fn arrow_left(&mut self) {
        self.select_all_on_type = false;
        if self.cursor_position > 0 {
            let prev_char_idx = self.input[..self.cursor_position].chars().last().map(|c| self.cursor_position - c.len_utf8());
            if let Some(idx) = prev_char_idx {
                self.cursor_position = idx;
            }
        }
    }

    pub fn arrow_right(&mut self) {
        self.select_all_on_type = false;
        if self.cursor_position < self.input.len() {
            let next_char_len = self.input[self.cursor_position..].chars().next().unwrap().len_utf8();
            self.cursor_position += next_char_len;
        }
    }

    pub fn arrow_up(&mut self) {
        self.select_all_on_type = false;
        if self.history.is_empty() { return; }
        
        let new_idx = match self.history_index {
            Some(idx) => if idx + 1 < self.history.len() { idx + 1 } else { idx },
            None => 0,
        };
        self.history_index = Some(new_idx);
        self.input = self.history[new_idx].clone();
        self.cursor_position = self.input.len();
    }

    pub fn arrow_down(&mut self) {
        self.select_all_on_type = false;
        if let Some(idx) = self.history_index {
            if idx > 0 {
                let new_idx = idx - 1;
                self.history_index = Some(new_idx);
                self.input = self.history[new_idx].clone();
                self.cursor_position = self.input.len();
            } else {
                self.history_index = None;
                self.input.clear();
                self.cursor_position = 0;
            }
        }
    }

    pub fn push_history(&mut self, url: String) {
        if self.history.front() != Some(&url) {
            self.history.push_front(url);
            if self.history.len() > 16 {
                self.history.pop_back();
            }
        }
    }
}

fn minimal_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn resolve_navigation_target(input: &str, search_engine: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() { return String::new(); }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("file://") || trimmed.starts_with("magma://") {
        return trimmed.to_string();
    }

    if trimmed.starts_with("localhost:") || trimmed.starts_with("127.0.0.1") {
        return format!("http://{}", trimmed);
    }

    let looks_like_domain = trimmed.contains('.') && !trimmed.contains(' ');
    if looks_like_domain {
        return format!("https://{}", trimmed);
    }

    search_engine.replace("{}", &minimal_encode(trimmed))
}

pub fn render_omnibox(buffer: &mut [u32], width: usize, state: &OmniboxState, current_url: &str) {
    let bg_color = 0xFF_18_18_18; 
    let field_bg = if state.is_focused { 0xFF_00_00_00 } else { 0xFF_20_20_20 };
    let text_color = 0xFF_E0_E0_E0;
    let selected_bg = 0xFF_00_55_AA;

    crate::ui::clear_rect(buffer, width, 0, crate::ui::TABBAR_HEIGHT as usize, width, crate::ui::OMNIBOX_HEIGHT as usize, bg_color);

    let padding_x = 10;
    let padding_y = crate::ui::TABBAR_HEIGHT as usize + 8;
    let field_w = width.saturating_sub(20);

    crate::ui::clear_rect(buffer, width, padding_x, padding_y - 2, field_w, 20, field_bg);

    let display_text = if state.is_focused { &state.input } else { current_url };

    if state.is_focused && state.select_all_on_type && !display_text.is_empty() {
        let sel_w = (display_text.chars().count() * 8).min(field_w - 10);
        crate::ui::clear_rect(buffer, width, padding_x + 5, padding_y - 1, sel_w, 18, selected_bg);
    }

    crate::ui::draw_string(buffer, width, padding_x + 5, padding_y, display_text, text_color, field_w - 10);

    if state.is_focused && !state.select_all_on_type {
        let chars_before_cursor = state.input[..state.cursor_position].chars().count();
        let cursor_x = padding_x + 5 + (chars_before_cursor * 8);
        if cursor_x < width - 10 {
            crate::ui::clear_rect(buffer, width, cursor_x, padding_y, 2, 16, 0xFF_FF_FF_FF);
        }
    }
}
