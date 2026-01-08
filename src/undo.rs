use crate::command::CommandPart;

/// Represents an action that can be undone or redone.
///
/// Each variant stores the necessary information to reverse or replay the action.
pub enum UndoAction {
    /// A component was inserted at the given position.
    ///
    /// To undo: delete the component at `position`.
    /// To redo: re-insert the component at `position`.
    Insert { position: usize },

    /// A component's value was edited.
    ///
    /// To undo: restore `original_value` at `position`.
    /// To redo: apply `updated_value` at `position`.
    Edit {
        position: usize,
        original_value: String,
        updated_value: String,
    },

    /// A component was deleted from the given position.
    ///
    /// To undo: re-insert `deleted_value` at `position`.
    /// To redo: delete the component at `position`.
    Delete {
        position: usize,
        deleted_value: CommandPart,
    },
}

/// Manages undo and redo stacks for tracking reversible actions.
///
/// Uses two stacks to implement standard undo/redo behavior:
/// - `undo_stack`: actions that can be undone (most recent at the top)
/// - `redo_stack`: actions that have been undone and can be redone
#[derive(Default)]
pub struct Undo {
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
}

impl Undo {
    /// Pushes an action onto the undo stack.
    ///
    /// If `clear_redo` is `true`, the redo stack is cleared. This should be set
    /// to `true` for new user actions (to invalidate the redo history), and `false`
    /// when pushing as part of a redo operation.
    pub fn push(&mut self, action: UndoAction, clear_redo: bool) {
        self.undo_stack.push(action);
        if clear_redo {
            self.redo_stack.clear();
        }
    }

    /// Pops and returns the most recent action from the undo stack.
    ///
    /// Returns `None` if the undo stack is empty.
    pub fn pop(&mut self) -> Option<UndoAction> {
        self.undo_stack.pop()
    }

    /// Pushes an action onto the redo stack.
    ///
    /// This is typically called after an undo operation to allow redoing the action.
    pub fn push_redo(&mut self, action: UndoAction) {
        self.redo_stack.push(action);
    }

    /// Pops and returns the most recent action from the redo stack.
    ///
    /// Returns `None` if the redo stack is empty.
    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        self.redo_stack.pop()
    }
}
