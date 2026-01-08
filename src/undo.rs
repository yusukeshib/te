use crate::command::CommandPart;

// Each undo action contains the necessary information for undo/redo operations.
pub enum UndoAction {
    Insert {
        position: usize,
    },
    Edit {
        position: usize,
        original_value: String,
        updated_value: String,
    },
    Delete {
        position: usize,
        deleted_value: CommandPart,
    },
}

#[derive(Default)]
pub struct Undo {
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
}

impl Undo {
    pub fn push(&mut self, action: UndoAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }
    pub fn pop(&mut self) -> Option<UndoAction> {
        self.undo_stack.pop()
    }
    pub fn push_redo(&mut self, action: UndoAction) {
        self.redo_stack.push(action);
    }
    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        self.redo_stack.pop()
    }
}
