use crate::{
    command::{Command, CommandPart},
    undo::{Undo, UndoAction},
};
use ratatui::widgets::ListState;

pub struct App {
    pub cmd: Command,
    pub list_state: ListState,
    pub input_mode: bool,
    pub current_input: String,
    pub undo: Undo,
    pub cursor_y: u16,
}

impl App {
    pub fn new(cmd: Command, cursor_y: u16) -> Self {
        Self {
            cmd,
            list_state: ListState::default().with_selected(Some(0)),
            input_mode: false,
            current_input: String::new(),
            undo: Undo::new(),
            cursor_y,
        }
    }

    pub fn undo(&mut self) {
        if let Some(last) = self.undo.pop() {
            match last {
                UndoAction::Insert {
                    position,
                    inserted_value,
                } => {
                    self.cmd.remove_component_at(position);
                    let count = self.cmd.component_count();
                    if count == 0 {
                        self.list_state.select(None);
                    } else if position >= count {
                        self.list_state.select(Some(count - 1));
                    } else {
                        self.list_state.select(Some(position));
                    }
                    self.undo.push_redo(UndoAction::Insert {
                        position,
                        inserted_value,
                    });
                }
                UndoAction::Edit {
                    position,
                    original_value,
                    updated_value,
                } => {
                    self.cmd.set_value_at(position, &original_value);
                    self.list_state.select(Some(position));
                    self.undo.push_redo(UndoAction::Edit {
                        position,
                        original_value,
                        updated_value,
                    });
                }
                UndoAction::Delete {
                    position,
                    deleted_value,
                } => {
                    self.cmd
                        .insert_component_at(position, deleted_value.clone());
                    self.list_state.select(Some(position));
                    self.undo.push_redo(UndoAction::Delete {
                        position,
                        deleted_value,
                    });
                }
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.undo.pop_redo() {
            match action {
                UndoAction::Insert {
                    position,
                    inserted_value,
                } => {
                    self.cmd.insert_component_at(
                        position,
                        crate::command::CommandPart::Value(inserted_value.clone()),
                    );
                    self.list_state.select(Some(position));
                    self.undo.actions.push(UndoAction::Insert {
                        position,
                        inserted_value,
                    });
                }
                UndoAction::Edit {
                    position,
                    original_value,
                    updated_value,
                } => {
                    self.cmd.set_value_at(position, &updated_value);
                    self.list_state.select(Some(position));
                    self.undo.actions.push(UndoAction::Edit {
                        position,
                        original_value,
                        updated_value,
                    });
                }
                UndoAction::Delete {
                    position,
                    deleted_value,
                } => {
                    self.cmd.remove_component_at(position);
                    let count = self.cmd.component_count();
                    if count == 0 {
                        self.list_state.select(None);
                    } else if position >= count {
                        self.list_state.select(Some(count - 1));
                    } else {
                        self.list_state.select(Some(position));
                    }
                    self.undo.actions.push(UndoAction::Delete {
                        position,
                        deleted_value,
                    });
                }
            }
        }
    }

    pub fn insert_new_component(&mut self) {
        let insert_at = match self.list_state.selected() {
            Some(i) => i,
            None => 0,
        };
        self.insert_new_component_at(insert_at);
    }

    pub fn append_new_component(&mut self) {
        let insert_at = match self.list_state.selected() {
            Some(i) => i + 1,
            None => self.cmd.component_count(),
        };
        self.insert_new_component_at(insert_at);
    }

    fn insert_new_component_at(&mut self, insert_at: usize) {
        let value = "".to_string();
        self.cmd
            .insert_component_at(insert_at, CommandPart::Value(value.clone()));
        self.list_state.select(Some(insert_at));

        self.undo.push(UndoAction::Insert {
            position: insert_at,
            inserted_value: value,
        })
    }

    pub fn delete_selected_component(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            let value = self.cmd.remove_component_at(selected);

            self.undo.push(UndoAction::Delete {
                position: selected,
                deleted_value: value,
            });

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
            let old_value = self.cmd.set_value_at(selected, &self.current_input);

            self.undo.push(UndoAction::Edit {
                position: selected,
                original_value: old_value,
                updated_value: self.current_input.clone(),
            })
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

    #[test]
    fn test_insert_at_beginning() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(0));

        app.insert_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.cmd.component_at(0).as_str(), "");
        assert_eq!(app.cmd.component_at(1).as_str(), "kubectl");
    }

    #[test]
    fn test_insert_at_middle() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(1));

        app.insert_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(1));
        assert_eq!(app.cmd.component_at(0).as_str(), "kubectl");
        assert_eq!(app.cmd.component_at(1).as_str(), "");
        assert_eq!(app.cmd.component_at(2).as_str(), "get");
    }

    #[test]
    fn test_insert_with_no_selection() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(None);

        app.insert_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.cmd.component_at(0).as_str(), "");
    }

    #[test]
    fn test_append_at_beginning() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(0));

        app.append_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(1));
        assert_eq!(app.cmd.component_at(0).as_str(), "kubectl");
        assert_eq!(app.cmd.component_at(1).as_str(), "");
        assert_eq!(app.cmd.component_at(2).as_str(), "get");
    }

    #[test]
    fn test_append_at_middle() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(1));

        app.append_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(2));
        assert_eq!(app.cmd.component_at(1).as_str(), "get");
        assert_eq!(app.cmd.component_at(2).as_str(), "");
        assert_eq!(app.cmd.component_at(3).as_str(), "pods");
    }

    #[test]
    fn test_append_at_end() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(Some(2));

        app.append_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(3));
        assert_eq!(app.cmd.component_at(3).as_str(), "");
    }

    #[test]
    fn test_append_with_no_selection() {
        let mut app = create_app("kubectl get pods");
        app.list_state.select(None);

        app.append_new_component();

        assert_eq!(app.cmd.component_count(), 4);
        assert_eq!(app.list_state.selected(), Some(3));
        assert_eq!(app.cmd.component_at(3).as_str(), "");
    }
}
