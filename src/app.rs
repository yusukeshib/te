use ratatui::widgets::ListState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandComponent {
    Base(String),
    StringArgument(String, String),
    BoolArgument(String, bool),
}

pub struct App {
    pub components: Vec<CommandComponent>,
    pub list_state: ListState,
    pub preview_command: String,
    pub input_mode: bool,
    pub current_input: String,
    pub history_options: HashMap<usize, Vec<String>>,
    pub current_option_index: HashMap<usize, usize>,
    pub cursor_y: u16,
}

impl App {
    pub fn new(
        components: Vec<CommandComponent>,
        history: HashMap<String, Vec<String>>,
        cursor_y: u16,
    ) -> Self {
        let mut list_state = ListState::default();
        if !components.is_empty() {
            list_state.select(Some(0));
        }

        let preview_command = Self::build_preview(&components);

        // Build mapping of history options
        let mut history_options = HashMap::new();
        let mut current_option_index = HashMap::new();

        for (idx, component) in components.iter().enumerate() {
            match component {
                CommandComponent::StringArgument(flag, current) => {
                    if let Some(values) = history.get(flag) {
                        if !values.is_empty() {
                            history_options.insert(idx, values.clone());
                            // Set index if current value exists in history
                            let option_idx = values.iter().position(|v| v == current).unwrap_or(0);
                            current_option_index.insert(idx, option_idx);
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            components,
            list_state,
            preview_command,
            input_mode: false,
            current_input: String::new(),
            history_options,
            current_option_index,
            cursor_y,
        }
    }

    fn build_preview(components: &[CommandComponent]) -> String {
        let mut parts = Vec::new();

        for component in components.iter() {
            match component {
                CommandComponent::Base(s) => {
                    parts.push(s.clone());
                }
                CommandComponent::StringArgument(flag, value) => {
                    if !flag.is_empty() {
                        parts.push(flag.clone());
                    }
                    parts.push(value.clone());
                }
                CommandComponent::BoolArgument(flag, checked) => {
                    if *checked {
                        parts.push(flag.clone());
                    }
                }
            }
        }

        parts.join(" ")
    }

    pub fn update_preview(&mut self) {
        self.preview_command = Self::build_preview(&self.components);
    }

    pub fn next(&mut self) {
        if self.components.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.components.len() - 1 {
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
        if self.components.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.components.len() - 1
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
            match &self.components[selected] {
                CommandComponent::Base(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                CommandComponent::StringArgument(_, value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                _ => {}
            }
        }
    }

    pub fn confirm_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.components[selected] {
                CommandComponent::Base(_) => {
                    self.components[selected] = CommandComponent::Base(self.current_input.clone());
                    self.update_preview();
                }
                CommandComponent::StringArgument(flag, _) => {
                    self.components[selected] =
                        CommandComponent::StringArgument(flag.clone(), self.current_input.clone());
                    self.update_preview();
                }
                _ => {}
            }
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
            if let CommandComponent::BoolArgument(flag, checked) = &self.components[selected] {
                self.components[selected] = CommandComponent::BoolArgument(flag.clone(), !checked);
                self.update_preview();
            }
        }
    }

    pub fn handle_enter(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.components[selected] {
                CommandComponent::Base(_) => self.start_input(),
                CommandComponent::StringArgument(_, _) => self.start_input(),
                CommandComponent::BoolArgument(_, _) => self.toggle_checkbox(),
            }
        }
    }

    pub fn next_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(
                self.components[selected],
                CommandComponent::StringArgument(_, _)
            ) {
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
                if let CommandComponent::StringArgument(flag, _) = &self.components[selected] {
                    self.components[selected] =
                        CommandComponent::StringArgument(flag.clone(), options[next_idx].clone());
                }
                self.update_preview();
            }
        }
    }

    pub fn previous_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(
                self.components[selected],
                CommandComponent::StringArgument(_, _)
            ) {
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
                if let CommandComponent::StringArgument(flag, _) = &self.components[selected] {
                    self.components[selected] =
                        CommandComponent::StringArgument(flag.clone(), options[prev_idx].clone());
                }
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
