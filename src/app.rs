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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_app(cmd_str: &str) -> App {
        let cmd: Command = cmd_str.try_into().unwrap();
        App::new(cmd, 0)
    }

    #[test]
    fn test_delete_middle_component() {
        let mut app = create_app("kubectl get pods -n default");
        // Select index 2 ("pods")
        app.list_state.select(Some(2));

        app.delete_selected_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(2));
        // Now index 2 should be "-n"
        assert_eq!(app.cmd.component_at(2).as_str(), "-n");
    }

    #[test]
    fn test_delete_last_component() {
        let mut app = create_app("kubectl get pods");
        // Select last component (index 2)
        app.list_state.select(Some(2));

        app.delete_selected_component();

        assert_eq!(app.cmd.component_count(), 2);
        // Selection should move to new last item (index 1)
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn test_delete_only_component() {
        let mut app = create_app("kubectl");
        app.list_state.select(Some(0));

        app.delete_selected_component();

        assert_eq!(app.cmd.component_count(), 0);
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn test_delete_with_no_selection() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(None);

        let count_before = app.cmd.component_count();
        app.delete_selected_component();

        // Nothing should change
        assert_eq!(app.cmd.component_count(), count_before);
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn test_delete_first_component() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(0));

        app.delete_selected_component();

        assert_eq!(app.cmd.component_count(), 2);
        // Selection stays at 0
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.cmd.component_at(0).as_str(), "get");
    }
}
