use crate::command::{Command, Comp};
use ratatui::widgets::ListState;
use std::collections::HashMap;

pub struct App {
    pub cmd: Command,
    pub list_state: ListState,
    pub input_mode: bool,
    pub current_input: String,
    pub history_options: HashMap<usize, Vec<String>>,
    pub current_option_index: HashMap<usize, usize>,
    pub cursor_y: u16,
}

impl App {
    pub fn new(cmd: Command, history: HashMap<String, Vec<String>>, cursor_y: u16) -> Self {
        let mut list_state = ListState::default();
        if !components.is_empty() {
            // Select first non-LineBreak component
            let first_selectable = components
                .iter()
                .position(|c| !matches!(c, Comp::LineBreak));
            if let Some(idx) = first_selectable {
                list_state.select(Some(idx));
            }
        }

        // Build mapping of history options for Value components
        let mut history_options = HashMap::new();
        let mut current_option_index = HashMap::new();

        // Look for Flag followed by Value to build history (skip LineBreaks)
        for idx in 0..components.len() {
            if let Comp::Value(current) = &components[idx] {
                // Find previous non-LineBreak component
                let mut prev_idx = idx;
                while prev_idx > 0 {
                    prev_idx -= 1;
                    match &components[prev_idx] {
                        Comp::LineBreak => continue,
                        Comp::Flag(flag) => {
                            if let Some(values) = history.get(flag) {
                                let mut options = values.clone();
                                // Ensure current value is in the options list
                                if !options.contains(current) {
                                    options.push(current.clone());
                                }
                                if !options.is_empty() {
                                    let option_idx =
                                        options.iter().position(|v| v == current).unwrap_or(0);
                                    history_options.insert(idx, options);
                                    current_option_index.insert(idx, option_idx);
                                }
                            }
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }

        Self {
            cmd,
            list_state,
            input_mode: false,
            current_input: String::new(),
            history_options,
            current_option_index,
            cursor_y,
        }
    }

    pub fn select_next_component(&mut self) {
        if self.components.is_empty() {
            return;
        }
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };

        // Find next non-LineBreak component
        let mut i = start;
        loop {
            i = if i >= self.components.len() - 1 {
                0
            } else {
                i + 1
            };
            if i == start {
                // Wrapped around to start, no selectable components
                return;
            }
            if !matches!(self.components[i], Comp::LineBreak) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    pub fn select_previous_component(&mut self) {
        if self.components.is_empty() {
            return;
        }
        let start = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };

        // Find previous non-LineBreak component
        let mut i = start;
        loop {
            i = if i == 0 {
                self.components.len() - 1
            } else {
                i - 1
            };
            if i == start {
                // Wrapped around to start, no selectable components
                return;
            }
            if !matches!(self.components[i], Comp::LineBreak) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    pub fn start_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.components[selected] {
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
                Comp::LineBreak => {
                    // LineBreak components should never be selected
                    unreachable!("LineBreak components should be skipped in navigation")
                }
            }
        }
    }

    pub fn confirm_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.components[selected] {
                Comp::Base(_) => {
                    self.components[selected] = Comp::Base(self.current_input.clone());
                    self.update_preview();
                }
                Comp::Flag(_) => {
                    self.components[selected] = Comp::Flag(self.current_input.clone());
                    self.update_preview();
                }
                Comp::Value(_) => {
                    self.components[selected] = Comp::Value(self.current_input.clone());
                    self.update_preview();
                }
                Comp::LineBreak => {
                    // LineBreak components should never be selected
                    unreachable!("LineBreak components should be skipped in navigation")
                }
            }
        }
        self.input_mode = false;
        self.current_input.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.current_input.clear();
    }

    pub fn select_next_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.components[selected], Comp::Value(_)) {
                return;
            }

            if let Some(options) = self.history_options.get(&selected) {
                if options.is_empty() {
                    return;
                }

                let current_idx = self
                    .current_option_index
                    .get(&selected)
                    .copied()
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % options.len();

                self.current_option_index.insert(selected, next_idx);
                self.components[selected] = Comp::Value(options[next_idx].clone());
                self.update_preview();
            }
        }
    }

    pub fn select_previous_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.components[selected], Comp::Value(_)) {
                return;
            }

            if let Some(options) = self.history_options.get(&selected) {
                if options.is_empty() {
                    return;
                }

                let current_idx = self
                    .current_option_index
                    .get(&selected)
                    .copied()
                    .unwrap_or(0);
                let prev_idx = if current_idx == 0 {
                    options.len() - 1
                } else {
                    current_idx - 1
                };

                self.current_option_index.insert(selected, prev_idx);
                self.components[selected] = Comp::Value(options[prev_idx].clone());
                self.update_preview();
            }
        }
    }
}
