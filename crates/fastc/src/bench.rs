//! Compile-time budget benchmark runner.
//!
//! Reads `compile-time-budget.toml` at the project root, executes each
//! benchmark `runs_per_benchmark` times (plus `warmup_runs` warmups),
//! aggregates the timings, compares against the declared target, and emits
//! both a JSON artifact and a human-readable markdown table.
//!
//! Consumed by the CI budget gate (see `.github/workflows/budget.yml`) and
//! reproducible locally with `fastc bench`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::diag::CompileError;

/// Top-level shape of `compile-time-budget.toml`.
#[derive(Debug, Deserialize)]
pub struct BudgetConfig {
    #[serde(default)]
    pub budgets: BTreeMap<String, BudgetEntry>,
    #[serde(default)]
    pub measurement: MeasurementConfig,
    #[serde(default)]
    pub reporting: ReportingConfig,
}

#[derive(Debug, Deserialize)]
pub struct BudgetEntry {
    pub description: String,
    pub target_ms: u64,
    #[serde(default = "default_threshold")]
    pub regression_threshold: f64,
    #[serde(default)]
    pub inputs: Option<String>,
    pub mode: BudgetMode,
}

fn default_threshold() -> f64 {
    0.20
}

/// What the benchmark actually does. Each variant is implemented by
/// `run_one_mode` below.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum BudgetMode {
    /// Compile every input file to C. Closest to `fastc compile` on a project.
    Compile,
    /// Compile in `--dev` mode (tcc backend when available).
    CompileDev,
    /// Cold-cache `fastc check` on every input file.
    CheckCold,
    /// Run `fastc check` once (warmup) then again, measuring the second run.
    /// Currently a placeholder; produces the same number as CheckCold until
    /// Salsa caching lands.
    CheckWarm,
    /// `cargo build --release -p fastc`.
    CargoBuildRelease,
}

#[derive(Debug, Deserialize)]
pub struct MeasurementConfig {
    #[serde(default = "default_runs")]
    pub runs_per_benchmark: u32,
    #[serde(default = "default_warmup")]
    pub warmup_runs: u32,
    #[serde(default = "default_metric")]
    pub report_metric: String,
}

impl Default for MeasurementConfig {
    fn default() -> Self {
        Self {
            runs_per_benchmark: default_runs(),
            warmup_runs: default_warmup(),
            report_metric: default_metric(),
        }
    }
}

fn default_runs() -> u32 {
    5
}
fn default_warmup() -> u32 {
    2
}
fn default_metric() -> String {
    "min".to_string()
}

#[derive(Debug, Deserialize, Default)]
pub struct ReportingConfig {
    #[serde(default)]
    pub emit_json: Option<PathBuf>,
    #[serde(default)]
    pub emit_markdown: Option<PathBuf>,
}

/// Outcome of one measured benchmark.
#[derive(Debug, Serialize)]
pub struct BenchResult {
    pub name: String,
    pub description: String,
    pub mode: String,
    pub target_ms: u64,
    pub measured_ms: u64,
    pub regression_threshold: f64,
    pub status: BudgetStatus,
    pub all_runs_ms: Vec<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BudgetStatus {
    /// Measured at or under target.
    Pass,
    /// Over target, within the regression threshold.
    Warn,
    /// Over target by more than `regression_threshold`. Fails CI.
    Fail,
    /// Benchmark could not be executed (skipped, missing inputs, etc.).
    Skip,
}

/// Top-level bench report — what `fastc bench` writes to disk.
#[derive(Debug, Serialize)]
pub struct BenchReport {
    pub fastc_version: String,
    pub host: String,
    pub results: Vec<BenchResult>,
    pub overall_status: BudgetStatus,
}

impl BenchReport {
    /// Render the report as a markdown table identical in shape to the CI
    /// comment the budget gate posts.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# fastC compile-time budget\n\n");
        out.push_str(&format!(
            "fastc {} | host: {}\n\n",
            self.fastc_version, self.host
        ));
        out.push_str("| Benchmark | Target | Measured | Δ | Status |\n");
        out.push_str("|-----------|-------:|---------:|--:|:------:|\n");
        for r in &self.results {
            let delta_pct = if r.target_ms == 0 {
                0.0
            } else {
                ((r.measured_ms as f64 - r.target_ms as f64) / r.target_ms as f64) * 100.0
            };
            let status_icon = match r.status {
                BudgetStatus::Pass => "✓",
                BudgetStatus::Warn => "⚠",
                BudgetStatus::Fail => "✗",
                BudgetStatus::Skip => "—",
            };
            out.push_str(&format!(
                "| {} | {}ms | {}ms | {:+.1}% | {} |\n",
                r.name, r.target_ms, r.measured_ms, delta_pct, status_icon
            ));
        }
        out.push_str(&format!("\nOverall: {:?}\n", self.overall_status));
        out
    }

    /// Render as pretty-printed JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Locate `compile-time-budget.toml` starting from `start_dir`, walking up.
pub fn find_budget_toml(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir;
    loop {
        let candidate = dir.join("compile-time-budget.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

/// Parse the budget TOML at `path`.
pub fn load_budget(path: &Path) -> Result<BudgetConfig, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    toml::from_str::<BudgetConfig>(&text).map_err(|e| format!("parse {}: {}", path.display(), e))
}

/// Run every benchmark declared in `config`, in deterministic order, and
/// return an aggregated `BenchReport`.
pub fn run_all(config: &BudgetConfig, project_root: &Path) -> BenchReport {
    let mut results = Vec::with_capacity(config.budgets.len());
    let mut any_fail = false;
    let mut any_warn = false;

    for (name, entry) in &config.budgets {
        let result = run_one(name, entry, &config.measurement, project_root);
        match result.status {
            BudgetStatus::Fail => any_fail = true,
            BudgetStatus::Warn => any_warn = true,
            _ => {}
        }
        results.push(result);
    }

    let overall = if any_fail {
        BudgetStatus::Fail
    } else if any_warn {
        BudgetStatus::Warn
    } else {
        BudgetStatus::Pass
    };

    BenchReport {
        fastc_version: env!("CARGO_PKG_VERSION").to_string(),
        host: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        results,
        overall_status: overall,
    }
}

fn run_one(
    name: &str,
    entry: &BudgetEntry,
    measurement: &MeasurementConfig,
    project_root: &Path,
) -> BenchResult {
    let mode_str = format!("{:?}", entry.mode);
    let inputs = resolve_inputs(entry.inputs.as_deref(), project_root);

    // Warmup: ignore timing, just trigger any first-time costs.
    for _ in 0..measurement.warmup_runs {
        let _ = run_one_mode(entry.mode, &inputs, project_root);
    }

    let mut runs_ms: Vec<u64> = Vec::with_capacity(measurement.runs_per_benchmark as usize);
    let mut last_err: Option<String> = None;
    for _ in 0..measurement.runs_per_benchmark {
        match run_one_mode(entry.mode, &inputs, project_root) {
            Ok(ms) => runs_ms.push(ms),
            Err(e) => last_err = Some(e),
        }
    }

    if runs_ms.is_empty() {
        eprintln!(
            "  {}: skipped ({})",
            name,
            last_err.unwrap_or_else(|| "no runs completed".into())
        );
        return BenchResult {
            name: name.to_string(),
            description: entry.description.clone(),
            mode: mode_str,
            target_ms: entry.target_ms,
            measured_ms: 0,
            regression_threshold: entry.regression_threshold,
            status: BudgetStatus::Skip,
            all_runs_ms: vec![],
        };
    }

    let measured = aggregate(&runs_ms, &measurement.report_metric);
    let status = classify(measured, entry.target_ms, entry.regression_threshold);

    eprintln!(
        "  {}: {}ms (target {}ms, {:?})",
        name, measured, entry.target_ms, status
    );

    BenchResult {
        name: name.to_string(),
        description: entry.description.clone(),
        mode: mode_str,
        target_ms: entry.target_ms,
        measured_ms: measured,
        regression_threshold: entry.regression_threshold,
        status,
        all_runs_ms: runs_ms,
    }
}

fn resolve_inputs(pattern: Option<&str>, project_root: &Path) -> Vec<PathBuf> {
    let Some(pat) = pattern else {
        return vec![];
    };

    // Tiny glob: only support `<dir>/**/*.fc`, `<dir>/*.fc`, or an exact file
    // path. Anything more elaborate is a CI surface we don't need yet and a
    // full glob crate is an unnecessary dependency.
    let pat_path = project_root.join(pat);
    if pat.contains("**") {
        if let Some(prefix) = pat.split("/**").next() {
            return walk_fc_files(&project_root.join(prefix));
        }
    } else if pat.ends_with("/*.fc") {
        if let Some(prefix) = pat.strip_suffix("/*.fc") {
            return list_fc_files(&project_root.join(prefix));
        }
    } else if pat_path.is_file() {
        return vec![pat_path];
    }
    vec![]
}

fn walk_fc_files(root: &Path) -> Vec<PathBuf> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|e| e == "fc") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    visit(root, &mut out);
    out.sort();
    out
}

fn list_fc_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "fc"))
        .collect();
    out.sort();
    out
}

fn run_one_mode(mode: BudgetMode, inputs: &[PathBuf], project_root: &Path) -> Result<u64, String> {
    let start = Instant::now();
    match mode {
        BudgetMode::Compile | BudgetMode::CompileDev => {
            if inputs.is_empty() {
                return Err("no inputs matched the configured pattern".to_string());
            }
            // Per-file errors are tolerated: a benchmark is measuring time, not
            // correctness. Some example files intentionally fail to compile
            // (safety tests, overflow tests). We still want their lex/parse/
            // resolve time captured.
            for input in inputs {
                let source = match std::fs::read_to_string(input) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let filename = input.display().to_string();
                let _ = compile_in_process(&source, &filename);
            }
        }
        BudgetMode::CheckCold | BudgetMode::CheckWarm => {
            if inputs.is_empty() {
                return Err("no inputs matched the configured pattern".to_string());
            }
            for input in inputs {
                let source = match std::fs::read_to_string(input) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let filename = input.display().to_string();
                let _ = check_in_process(&source, &filename);
            }
        }
        BudgetMode::CargoBuildRelease => {
            let status = Command::new("cargo")
                .args(["build", "--release", "-p", "fastc"])
                .current_dir(project_root)
                .status()
                .map_err(|e| format!("spawn cargo: {}", e))?;
            if !status.success() {
                return Err("cargo build failed".to_string());
            }
        }
    }
    Ok(start.elapsed().as_millis() as u64)
}

fn compile_in_process(source: &str, filename: &str) -> Result<(), String> {
    crate::compile_with_options(source, filename, false)
        .map(|_| ())
        .map_err(stringify_compile_err)
}

fn check_in_process(source: &str, filename: &str) -> Result<(), String> {
    crate::check(source, filename).map_err(stringify_compile_err)
}

fn stringify_compile_err(e: CompileError) -> String {
    format!("{:?}", e)
}

fn aggregate(samples: &[u64], metric: &str) -> u64 {
    match metric {
        "min" => *samples.iter().min().unwrap_or(&0),
        "max" => *samples.iter().max().unwrap_or(&0),
        "median" => {
            let mut sorted: Vec<u64> = samples.to_vec();
            sorted.sort_unstable();
            sorted[sorted.len() / 2]
        }
        _ => {
            // mean by default
            let sum: u64 = samples.iter().sum();
            sum / samples.len() as u64
        }
    }
}

fn classify(measured: u64, target: u64, threshold: f64) -> BudgetStatus {
    if measured <= target {
        BudgetStatus::Pass
    } else {
        let over = (measured as f64 - target as f64) / target.max(1) as f64;
        if over > threshold {
            BudgetStatus::Fail
        } else {
            BudgetStatus::Warn
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_at_target_is_pass() {
        assert_eq!(classify(100, 100, 0.20), BudgetStatus::Pass);
    }

    #[test]
    fn classify_within_threshold_is_warn() {
        // 110 vs target 100 = +10%, threshold 0.20 = +20%.
        assert_eq!(classify(110, 100, 0.20), BudgetStatus::Warn);
    }

    #[test]
    fn classify_over_threshold_is_fail() {
        // 150 vs target 100 = +50%, exceeds 0.20.
        assert_eq!(classify(150, 100, 0.20), BudgetStatus::Fail);
    }

    #[test]
    fn aggregate_min_picks_smallest() {
        assert_eq!(aggregate(&[10, 5, 20, 15], "min"), 5);
    }

    #[test]
    fn aggregate_median_middle() {
        assert_eq!(aggregate(&[1, 2, 3, 4, 5], "median"), 3);
    }

    #[test]
    fn budget_toml_parses() {
        let s = r#"
[budgets.test]
description = "demo"
target_ms = 100
mode = "check_cold"
inputs = "examples/foo.fc"
"#;
        let cfg: BudgetConfig = toml::from_str(s).unwrap();
        assert_eq!(cfg.budgets.len(), 1);
        let entry = cfg.budgets.get("test").unwrap();
        assert_eq!(entry.target_ms, 100);
        assert_eq!(entry.mode, BudgetMode::CheckCold);
    }
}
