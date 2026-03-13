//! Unsafe tracking for safety checks

/// Tracks whether we're in an unsafe context
#[derive(Debug, Default)]
pub struct SafetyContext {
    /// Stack of unsafe contexts (true = in unsafe)
    unsafe_stack: Vec<bool>,
}

impl SafetyContext {
    pub fn new() -> Self {
        Self {
            unsafe_stack: vec![false], // Start in safe context
        }
    }

    /// Enter an unsafe block or function
    pub fn enter_unsafe(&mut self) {
        self.unsafe_stack.push(true);
    }

    /// Exit the current unsafe context
    pub fn exit_unsafe(&mut self) {
        self.unsafe_stack.pop();
    }

    /// Check if we're currently in an unsafe context
    pub fn is_unsafe(&self) -> bool {
        self.unsafe_stack.last().copied().unwrap_or(false)
    }
}
