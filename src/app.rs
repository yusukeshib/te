use ratatui::widgets::ListState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandComponent {
    Base(String),
    Flag(String),
    Value(String),
    LineBreak,
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

pub fn quote_if_needed(s: &str) -> String {
    if s.contains(' ') {
        // Escape existing double quotes
        let escaped = s.replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

impl App {
    pub fn new(
        components: Vec<CommandComponent>,
        history: HashMap<String, Vec<String>>,
        cursor_y: u16,
    ) -> Self {
        let mut list_state = ListState::default();
        if !components.is_empty() {
            // Select first non-LineBreak component
            let first_selectable = components
                .iter()
                .position(|c| !matches!(c, CommandComponent::LineBreak));
            if let Some(idx) = first_selectable {
                list_state.select(Some(idx));
            }
        }

        let preview_command = Self::build_preview(&components);

        // Build mapping of history options for Value components
        let mut history_options = HashMap::new();
        let mut current_option_index = HashMap::new();

        // Look for Flag followed by Value to build history
        for idx in 0..components.len() {
            if let CommandComponent::Value(current) = &components[idx] {
                // Check if previous component is a Flag
                if idx > 0 {
                    if let CommandComponent::Flag(flag) = &components[idx - 1] {
                        if let Some(values) = history.get(flag) {
                            if !values.is_empty() {
                                history_options.insert(idx, values.clone());
                                let option_idx =
                                    values.iter().position(|v| v == current).unwrap_or(0);
                                current_option_index.insert(idx, option_idx);
                            }
                        }
                    }
                }
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
                CommandComponent::Base(s) => parts.push(quote_if_needed(s)),
                CommandComponent::Flag(s) => parts.push(quote_if_needed(s)),
                CommandComponent::Value(s) => parts.push(quote_if_needed(s)),
                CommandComponent::LineBreak => {} // Skip line breaks in preview
            }
        }

        parts.join(" ")
    }

    pub fn update_preview(&mut self) {
        self.preview_command = Self::build_preview(&self.components);
    }

    pub fn build_final_command(&self) -> String {
        let mut result = String::new();

        for (idx, component) in self.components.iter().enumerate() {
            match component {
                CommandComponent::Base(s)
                | CommandComponent::Flag(s)
                | CommandComponent::Value(s) => {
                    if idx > 0
                        && !matches!(
                            self.components.get(idx - 1),
                            Some(CommandComponent::LineBreak)
                        )
                    {
                        result.push(' ');
                    }
                    result.push_str(&quote_if_needed(s));
                }
                CommandComponent::LineBreak => {
                    result.push_str(" \\\n");
                }
            }
        }

        result
    }

    pub fn next(&mut self) {
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
            if !matches!(self.components[i], CommandComponent::LineBreak) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    pub fn previous(&mut self) {
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
            if !matches!(self.components[i], CommandComponent::LineBreak) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    pub fn start_input(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match &self.components[selected] {
                CommandComponent::Base(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                CommandComponent::Flag(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                CommandComponent::Value(value) => {
                    self.input_mode = true;
                    self.current_input = value.clone();
                }
                CommandComponent::LineBreak => {
                    // LineBreak components should never be selected
                    unreachable!("LineBreak components should be skipped in navigation")
                }
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
                CommandComponent::Flag(_) => {
                    self.components[selected] = CommandComponent::Flag(self.current_input.clone());
                    self.update_preview();
                }
                CommandComponent::Value(_) => {
                    self.components[selected] = CommandComponent::Value(self.current_input.clone());
                    self.update_preview();
                }
                CommandComponent::LineBreak => {
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

    pub fn handle_enter(&mut self) {
        self.start_input();
    }

    pub fn next_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.components[selected], CommandComponent::Value(_)) {
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
                self.components[selected] = CommandComponent::Value(options[next_idx].clone());
                self.update_preview();
            }
        }
    }

    pub fn previous_option(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if !matches!(self.components[selected], CommandComponent::Value(_)) {
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
                self.components[selected] = CommandComponent::Value(options[prev_idx].clone());
                self.update_preview();
            }
        }
    }
}
