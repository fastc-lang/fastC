//! Temporary variable generation for evaluation order enforcement

/// Temporary generator
pub struct TempGen {
    counter: usize,
}

impl TempGen {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Generate a fresh temporary name
    pub fn fresh(&mut self, prefix: &str) -> String {
        let name = format!("__{}{}", prefix, self.counter);
        self.counter += 1;
        name
    }
}

impl Default for TempGen {
    fn default() -> Self {
        Self::new()
    }
}
