use ratatui::widgets::ListState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub flag: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Checked(bool),
}

pub struct App {
    pub base_command: Vec<String>,
    pub arguments: Vec<Argument>,
    pub list_state: ListState,
    pub preview_command: String,
    pub input_mode: bool,
    pub current_input: String,
    pub history_options: HashMap<usize, Vec<String>>,
    pub current_option_index: HashMap<usize, usize>,
}

impl App {
    pub fn new(
        base_command: Vec<String>,
        arguments: Vec<Argument>,
        history: HashMap<String, Vec<String>>,
    ) -> Self {
        let mut list_state = ListState::default();
        if !arguments.is_empty() {
            list_state.select(Some(0));
        }

        let preview_command = Self::build_preview(&base_command, &arguments);

        // Build mapping of history options
        let mut history_options = HashMap::new();
        let mut current_option_index = HashMap::new();

        for (idx, arg) in arguments.iter().enumerate() {
            if let Some(values) = history.get(&arg.flag) {
                if !values.is_empty() {
                    history_options.insert(idx, values.clone());
                    // Set index if current value exists in history
                    if let Value::String(current) = &arg.value {
                        let option_idx = values.iter().position(|v| v == current).unwrap_or(0);
                        current_option_index.insert(idx, option_idx);
                    } else {
                        current_option_index.insert(idx, 0);
                    }
                }
            }
        }

        Self {
            base_command,
            arguments,
            list_state,
            preview_command,
            input_mode: false,
            current_input: String::new(),
            history_options,
            current_option_index,
        }
    }

    fn build_preview(base_command: &[String], arguments: &[Argument]) -> String {
        let mut parts = base_command.to_vec();

        for arg in arguments.iter() {
            match &arg.value {
                Value::String(s) => {
                    if !arg.flag.is_empty() {
                        parts.push(arg.flag.clone());
                    }
                    parts.push(s.clone());
                }
                Value::Checked(checked) => {
                    if *checked {
                        parts.push(arg.flag.clone());
                    }
                }
            }
        }

        parts.join(" ")
    }

    pub fn update_preview(&mut self) {
        self.preview_command = Self::build_preview(&self.base_command, &self.arguments);
    }

    pub fn next(&mut self) {
        if self.arguments.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.arguments.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.arguments.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.arguments.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn start_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            // Only allow editing for String values
            if let Value::String(s) = &self.arguments[selected].value {
                self.input_mode = true;
                self.current_input = s.clone();
            }
        }
    }

    pub fn confirm_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            self.arguments[selected].value = Value::String(self.current_input.clone());
            self.update_preview();
        }
        self.input_mode = false;
        self.current_input.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.current_input.clear();
    }

    pub fn toggle_checkbox(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Value::Checked(checked) = &self.arguments[selected].value {
                self.arguments[selected].value = Value::Checked(!checked);
                self.update_preview();
            }
        }
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.arguments[selected].value {
                Value::String(_) => self.start_input(),
                Value::Checked(_) => self.toggle_checkbox(),
            }
        }
    }

    pub fn next_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.arguments[selected].value, Value::String(_)) {
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
                self.arguments[selected].value = Value::String(options[next_idx].clone());
                self.update_preview();
            }
        }
    }

    pub fn previous_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.arguments[selected].value, Value::String(_)) {
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
                self.arguments[selected].value = Value::String(options[prev_idx].clone());
                self.update_preview();
            }
        }
    }

    pub fn get_option_status(&self, arg_index: usize) -> Option<(usize, usize)> {
        if let Some(options) = self.history_options.get(&arg_index) {
            if options.is_empty() {
                return None;
            }
            let current = self
                .current_option_index
                .get(&arg_index)
                .copied()
                .unwrap_or(0);
            Some((current + 1, options.len()))
        } else {
            None
        }
    }
}
