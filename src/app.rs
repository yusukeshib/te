use crate::command::{Command, Comp};
use ratatui::widgets::ListState;

pub struct App {
    pub cmd: Command,
    pub list_state: ListState,
    pub input_mode: bool,
    pub current_input: String,
    pub cursor_y: u16,
}

impl App {
    pub fn new(cmd: Command, cursor_y: u16) -> Self {
        Self {
            cmd,
            list_state: ListState::default(),
            input_mode: false,
            current_input: String::new(),
            cursor_y,
        }
    }

    pub fn select_next_component(&mut self) {
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };

        // Find next non-LineBreak component
        let mut i = start;
        i = if i >= self.cmd.component_count() - 1 {
            0
        } else {
            i + 1
        };
        if i == start {
            // Wrapped around to start, no selectable components
            return;
        }
        self.list_state.select(Some(i));
        return;
    }

    pub fn select_previous_component(&mut self) {
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };
        // Find previous non-LineBreak component
        let mut i = start;
        i = if i == 0 {
            self.cmd.component_count() - 1
        } else {
            i - 1
        };
        if i == start {
            // Wrapped around to start, no selectable components
            return;
        }
        self.list_state.select(Some(i));
    }

    pub fn start_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match self.cmd.component_at(selected) {
                Comp::Base(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                Comp::Flag(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                Comp::Value(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
            }
        }
    }

    pub fn confirm_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            self.cmd.set_value_at(selected, &self.current_input);
        }
        self.input_mode = false;
        self.current_input.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.current_input.clear();
    }
}
