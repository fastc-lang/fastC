//! Power of 10 configuration
//!
//! Based on NASA/JPL's "Power of 10: Rules for Developing Safety-Critical Code"
//! by Gerard J. Holzmann.

/// Safety level for Power of 10 enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SafetyLevel {
    /// Standard FastC safety (default behavior)
    #[default]
    Standard,
    /// Full Power of 10 compliance mode for safety-critical code
    SafetyCritical,
    /// Relaxed mode for prototyping (minimal checks)
    Relaxed,
}

impl SafetyLevel {
    /// Parse safety level from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "standard" => Some(SafetyLevel::Standard),
            "critical" | "safety-critical" | "safetycritical" => Some(SafetyLevel::SafetyCritical),
            "relaxed" => Some(SafetyLevel::Relaxed),
            _ => None,
        }
    }
}

/// Configuration for Power of 10 rule enforcement
#[derive(Debug, Clone)]
pub struct P10Config {
    /// Safety level determines which rules are enforced
    pub level: SafetyLevel,

    /// Rule 4: Maximum lines per function (default: 60)
    pub max_function_lines: usize,

    /// Rule 5: Minimum assertions per function (default: 2)
    pub min_assertions_per_fn: usize,

    /// Rule 9: Maximum pointer dereference depth (default: 1)
    pub max_pointer_depth: usize,

    /// Rule 1: Allow recursion (default: false in SafetyCritical)
    pub allow_recursion: bool,

    /// Rule 2: Require provable loop bounds (default: true in SafetyCritical)
    pub require_loop_bounds: bool,

    /// Rule 3: Allow runtime memory allocation (default: false in SafetyCritical)
    pub allow_runtime_alloc: bool,

    /// Rule 10: Treat all warnings as errors
    pub strict_mode: bool,
}

impl Default for P10Config {
    fn default() -> Self {
        Self::standard()
    }
}

impl P10Config {
    /// Create configuration for standard safety level
    ///
    /// Enables key Power of 10 rules by default:
    /// - Rule 2: Loop bounds checking
    /// - Rule 3: No dynamic memory allocation
    /// - Rule 4: Function size limit (60 lines)
    /// - Rule 9: Single-level pointer dereferencing
    pub fn standard() -> Self {
        Self {
            level: SafetyLevel::Standard,
            max_function_lines: 60,
            min_assertions_per_fn: 0, // Not enforced in standard mode
            max_pointer_depth: 1,     // Single dereference level
            allow_recursion: true,    // Recursion allowed (Rule 1 not default)
            require_loop_bounds: true,
            allow_runtime_alloc: false,
            strict_mode: false,
        }
    }

    /// Create configuration for safety-critical level (full Power of 10)
    pub fn safety_critical() -> Self {
        Self {
            level: SafetyLevel::SafetyCritical,
            max_function_lines: 60,
            min_assertions_per_fn: 2,
            max_pointer_depth: 1,
            allow_recursion: false,
            require_loop_bounds: true,
            allow_runtime_alloc: false,
            strict_mode: true,
        }
    }

    /// Create configuration for relaxed level (prototyping)
    pub fn relaxed() -> Self {
        Self {
            level: SafetyLevel::Relaxed,
            max_function_lines: 200,
            min_assertions_per_fn: 0,
            max_pointer_depth: 10,
            allow_recursion: true,
            require_loop_bounds: false,
            allow_runtime_alloc: true,
            strict_mode: false,
        }
    }

    /// Create configuration from safety level
    pub fn from_level(level: SafetyLevel) -> Self {
        match level {
            SafetyLevel::Standard => Self::standard(),
            SafetyLevel::SafetyCritical => Self::safety_critical(),
            SafetyLevel::Relaxed => Self::relaxed(),
        }
    }

    /// Check if Power of 10 checking is enabled
    pub fn is_enabled(&self) -> bool {
        self.level != SafetyLevel::Relaxed
    }
}
