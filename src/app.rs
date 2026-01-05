use crate::command::Command;
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
            list_state: ListState::default().with_selected(Some(0)),
            input_mode: false,
            current_input: String::new(),
            cursor_y,
        }
    }

    pub fn delete_selected_component(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            self.cmd.remove_component_at(selected);
            let count = self.cmd.component_count();
            if count == 0 {
                self.list_state.select(None);
            } else if selected >= count {
                self.list_state.select(Some(count - 1));
            }
        }
    }

    pub fn select_next_component(&mut self) {
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };

        let mut i = start;
        i = if i >= self.cmd.component_count() - 1 {
            0
        } else {
            i + 1
        };
        self.list_state.select(Some(i));
    }

    pub fn select_previous_component(&mut self) {
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };
        let mut i = start;
        i = if i == 0 {
            self.cmd.component_count() - 1
        } else {
            i - 1
        };
        self.list_state.select(Some(i));
    }

    pub fn start_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            self.input_mode = true;
            self.current_input = self.cmd.component_at(selected).as_str().to_string();
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
