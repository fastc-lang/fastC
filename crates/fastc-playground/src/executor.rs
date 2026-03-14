use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Message types for execution output
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ExecutionMessage {
    #[serde(rename = "compile")]
    Compile { stage: String, output: String },
    #[serde(rename = "stdout")]
    Stdout { data: String },
    #[serde(rename = "stderr")]
    Stderr { data: String },
    #[serde(rename = "exit")]
    Exit { code: i32 },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Execute compiled C code in a sandboxed environment
pub struct Executor {
    work_dir: PathBuf,
    timeout: Duration,
}

impl Executor {
    pub fn new() -> Self {
        let work_dir = std::env::temp_dir().join("fastc-playground");
        std::fs::create_dir_all(&work_dir).ok();
        Self {
            work_dir,
            timeout: Duration::from_secs(5),
        }
    }

    /// Compile and run FastC code, streaming output to the broadcast channel
    pub async fn run(
        &self,
        session_id: Uuid,
        code: &str,
        tx: broadcast::Sender<ExecutionMessage>,
    ) -> Result<(), String> {
        // Create session directory
        let session_dir = self.work_dir.join(session_id.to_string());
        std::fs::create_dir_all(&session_dir).map_err(|e| e.to_string())?;

        let fc_file = session_dir.join("main.fc");
        let c_file = session_dir.join("main.c");
        let exe_file = session_dir.join("main");

        // Write FastC code
        std::fs::write(&fc_file, code).map_err(|e| e.to_string())?;

        // Compile FastC to C
        let _ = tx.send(ExecutionMessage::Compile {
            stage: "fastc".to_string(),
            output: "Compiling FastC...".to_string(),
        });

        let c_code = match fastc::compile(code, "main.fc") {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(ExecutionMessage::Error {
                    message: format!("FastC compilation failed: {}", e),
                });
                return Err(format!("{}", e));
            }
        };

        std::fs::write(&c_file, &c_code).map_err(|e| e.to_string())?;

        // Find C compiler
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

        // Find runtime include path
        let runtime_include = find_runtime_include();
        tracing::debug!("Runtime include path: {:?}", runtime_include);

        let _ = tx.send(ExecutionMessage::Compile {
            stage: "cc".to_string(),
            output: format!(
                "Compiling C with {}{}...",
                cc,
                runtime_include
                    .as_ref()
                    .map(|p| format!(" (runtime: {})", p.display()))
                    .unwrap_or_else(|| " (runtime not found!)".to_string())
            ),
        });

        // Compile C to executable
        let mut cc_cmd = Command::new(&cc);
        cc_cmd
            .arg("-o")
            .arg(&exe_file)
            .arg(&c_file)
            .arg("-Wall")
            .arg("-Wextra");

        if let Some(include) = &runtime_include {
            cc_cmd.arg(format!("-I{}", include.display()));
        }

        let cc_output = cc_cmd.output().map_err(|e| e.to_string())?;

        if !cc_output.status.success() {
            let stderr = String::from_utf8_lossy(&cc_output.stderr);
            let _ = tx.send(ExecutionMessage::Error {
                message: format!("C compilation failed:\n{}", stderr),
            });
            return Err(format!("C compilation failed: {}", stderr));
        }

        let _ = tx.send(ExecutionMessage::Compile {
            stage: "run".to_string(),
            output: "Running...".to_string(),
        });

        // Run the executable
        let mut child = Command::new(&exe_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Stream stdout
        let tx_stdout = tx.clone();
        let stdout_handle = tokio::task::spawn_blocking(move || {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx_stdout.send(ExecutionMessage::Stdout {
                            data: format!("{}\n", line),
                        });
                    }
                }
            }
        });

        // Stream stderr
        let tx_stderr = tx.clone();
        let stderr_handle = tokio::task::spawn_blocking(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx_stderr.send(ExecutionMessage::Stderr {
                            data: format!("{}\n", line),
                        });
                    }
                }
            }
        });

        // Wait with timeout
        let timeout = self.timeout;
        let wait_result = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => return Ok(status.code().unwrap_or(-1)),
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            child.kill().ok();
                            return Err("Execution timed out".to_string());
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
        })
        .await
        .map_err(|e| e.to_string())?;

        // Wait for output streams to finish
        stdout_handle.await.ok();
        stderr_handle.await.ok();

        match wait_result {
            Ok(code) => {
                let _ = tx.send(ExecutionMessage::Exit { code });
            }
            Err(msg) => {
                let _ = tx.send(ExecutionMessage::Error { message: msg });
            }
        }

        // Cleanup
        std::fs::remove_dir_all(&session_dir).ok();

        Ok(())
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the FastC runtime include directory
fn find_runtime_include() -> Option<PathBuf> {
    // Check environment variable
    if let Ok(path) = std::env::var("FASTC_RUNTIME") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // Check relative to executable (for installed binaries)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // Development: target/debug/fastc-playground -> runtime
            for depth in &["../../..", "../..", "..", ""] {
                let runtime = parent.join(depth).join("runtime");
                if runtime.join("fastc_runtime.h").exists() {
                    return Some(runtime.canonicalize().unwrap_or(runtime));
                }
            }
        }
    }

    // Check current working directory
    let cwd_runtime = PathBuf::from("runtime");
    if cwd_runtime.join("fastc_runtime.h").exists() {
        return Some(cwd_runtime.canonicalize().unwrap_or(cwd_runtime));
    }

    // Check relative to CARGO_MANIFEST_DIR (for development)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let runtime = PathBuf::from(manifest_dir).join("../../runtime");
        if runtime.join("fastc_runtime.h").exists() {
            return Some(runtime.canonicalize().unwrap_or(runtime));
        }
    }

    None
}
