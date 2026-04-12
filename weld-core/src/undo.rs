use std::collections::VecDeque;

/// A bounded undo/redo stack, generic over snapshot type `T`.
///
/// Pushing a new snapshot clears any pending redo entries (standard undo
/// semantics). When the stack exceeds `capacity`, the oldest entry is dropped.
#[derive(Debug)]
pub struct UndoStack<T> {
    undo: VecDeque<T>,
    redo: Vec<T>,
    capacity: usize,
}

impl<T> UndoStack<T> {
    /// Create a new stack with the given maximum capacity.
    ///
    /// # Panics
    /// Panics if `capacity` is 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "UndoStack capacity must be at least 1");
        UndoStack {
            undo: VecDeque::with_capacity(capacity),
            redo: Vec::new(),
            capacity,
        }
    }

    /// Push a snapshot onto the undo stack.
    /// Clears the redo stack (forking history).
    pub fn push(&mut self, snapshot: T) {
        self.redo.clear();
        if self.undo.len() == self.capacity {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
    }

    /// Pop the most recent snapshot from the undo stack.
    pub fn pop_undo(&mut self) -> Option<T> {
        self.undo.pop_back()
    }

    /// Pop the most recent snapshot from the redo stack.
    pub fn pop_redo(&mut self) -> Option<T> {
        self.redo.pop()
    }

    /// Push a snapshot onto the redo stack (used during undo to preserve current state).
    pub fn push_redo(&mut self, snapshot: T) {
        self.redo.push(snapshot);
    }

    /// Push a snapshot onto the undo stack without clearing redo (used during redo).
    pub fn push_undo(&mut self, snapshot: T) {
        if self.undo.len() == self.capacity {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
    }

    /// Whether there are entries available to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there are entries available to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_undo() {
        let mut stack = UndoStack::new(3);
        stack.push("a");
        stack.push("b");

        assert_eq!(stack.pop_undo(), Some("b"));
        assert_eq!(stack.pop_undo(), Some("a"));
        assert_eq!(stack.pop_undo(), None);
    }

    #[test]
    fn push_redo_and_pop_redo() {
        let mut stack = UndoStack::new(3);
        stack.push_redo("a");

        assert!(stack.can_redo());
        assert_eq!(stack.pop_redo(), Some("a"));
        assert!(!stack.can_redo());
    }

    #[test]
    fn push_clears_redo() {
        let mut stack = UndoStack::new(3);
        stack.push_redo("a");
        stack.push("b"); // redo cleared

        assert!(!stack.can_redo());
    }

    #[test]
    fn push_undo_does_not_clear_redo() {
        let mut stack = UndoStack::new(3);
        stack.push_redo("a");
        stack.push_undo("b");

        assert!(stack.can_redo());
        assert!(stack.can_undo());
    }

    #[test]
    fn capacity_drops_oldest() {
        let mut stack = UndoStack::new(2);
        stack.push("a");
        stack.push("b");
        stack.push("c"); // "a" dropped

        assert_eq!(stack.pop_undo(), Some("c"));
        assert_eq!(stack.pop_undo(), Some("b"));
        assert_eq!(stack.pop_undo(), None);
    }

    #[test]
    fn can_undo_and_redo() {
        let mut stack: UndoStack<&str> = UndoStack::new(2);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push("a");
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    #[should_panic(expected = "capacity must be at least 1")]
    fn zero_capacity_panics() {
        UndoStack::<()>::new(0);
    }
}
