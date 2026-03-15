use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
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
    limits: ExecutorLimits,
}

#[derive(Debug, Clone)]
pub struct ExecutorLimits {
    pub run_timeout: Duration,
    pub compile_timeout: Duration,
    pub max_output_bytes: usize,
    pub max_memory_bytes: u64,
    pub max_processes: u64,
}

impl Default for ExecutorLimits {
    fn default() -> Self {
        Self {
            run_timeout: Duration::from_secs(5),
            compile_timeout: Duration::from_secs(10),
            max_output_bytes: 64 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_processes: 32,
        }
    }
}

struct SessionDirGuard {
    path: PathBuf,
}

impl Drop for SessionDirGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path).ok();
    }
}

impl Executor {
    pub fn new(limits: ExecutorLimits) -> Self {
        let work_dir = std::env::temp_dir().join("fastc-playground");
        std::fs::create_dir_all(&work_dir).ok();
        Self { work_dir, limits }
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
        let _cleanup_guard = SessionDirGuard {
            path: session_dir.clone(),
        };

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

        cc_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        configure_process_group(&mut cc_cmd);
        let cc_output = run_command_with_timeout(cc_cmd, self.limits.compile_timeout)?;

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
        let mut run_cmd = Command::new(&exe_file);
        run_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        configure_run_command(&mut run_cmd, &self.limits);
        let mut child = run_cmd.spawn().map_err(|e| e.to_string())?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let output_budget = Arc::new(AtomicUsize::new(0));
        let did_truncate = Arc::new(AtomicBool::new(false));
        let max_output_bytes = self.limits.max_output_bytes;

        // Stream stdout
        let tx_stdout = tx.clone();
        let output_budget_stdout = output_budget.clone();
        let did_truncate_stdout = did_truncate.clone();
        let stdout_handle = tokio::task::spawn_blocking(move || {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let bytes = line.len() + 1;
                    let total = output_budget_stdout.fetch_add(bytes, Ordering::Relaxed) + bytes;
                    if total > max_output_bytes {
                        if !did_truncate_stdout.swap(true, Ordering::Relaxed) {
                            let _ = tx_stdout.send(ExecutionMessage::Error {
                                message: format!("Output truncated at {} bytes", max_output_bytes),
                            });
                        }
                        break;
                    }
                    let _ = tx_stdout.send(ExecutionMessage::Stdout {
                        data: format!("{}\n", line),
                    });
                }
            }
        });

        // Stream stderr
        let tx_stderr = tx.clone();
        let output_budget_stderr = output_budget;
        let did_truncate_stderr = did_truncate;
        let stderr_handle = tokio::task::spawn_blocking(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let bytes = line.len() + 1;
                    let total = output_budget_stderr.fetch_add(bytes, Ordering::Relaxed) + bytes;
                    if total > max_output_bytes {
                        if !did_truncate_stderr.swap(true, Ordering::Relaxed) {
                            let _ = tx_stderr.send(ExecutionMessage::Error {
                                message: format!("Output truncated at {} bytes", max_output_bytes),
                            });
                        }
                        break;
                    }
                    let _ = tx_stderr.send(ExecutionMessage::Stderr {
                        data: format!("{}\n", line),
                    });
                }
            }
        });

        // Wait with timeout
        let timeout = self.limits.run_timeout;
        let wait_result = tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => return Ok(status.code().unwrap_or(-1)),
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            kill_process_tree(&mut child);
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

        Ok(())
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new(ExecutorLimits::default())
    }
}

fn run_command_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Output, String> {
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|e| e.to_string()),
            Ok(None) => {
                if start.elapsed() > timeout {
                    kill_process_tree(&mut child);
                    return Err(format!("Command timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn kill_process_tree(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::killpg(child.id() as i32, libc::SIGKILL);
    }

    let _ = child.kill();
    let _ = child.wait();
}

fn configure_process_group(cmd: &mut Command) {
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }

    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}

fn configure_run_command(cmd: &mut Command, limits: &ExecutorLimits) {
    #[cfg(unix)]
    unsafe {
        let max_memory = limits.max_memory_bytes;
        let max_processes = limits.max_processes;
        let cpu_limit = limits.run_timeout.as_secs().saturating_add(1);

        cmd.pre_exec(move || {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            let cpu = libc::rlimit {
                rlim_cur: cpu_limit as libc::rlim_t,
                rlim_max: cpu_limit as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_CPU, &cpu) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                let mem = libc::rlimit {
                    rlim_cur: max_memory as libc::rlim_t,
                    rlim_max: max_memory as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &mem) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                let procs = libc::rlimit {
                    rlim_cur: max_processes as libc::rlim_t,
                    rlim_max: max_processes as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_NPROC, &procs) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            Ok(())
        });
    }

    #[cfg(not(unix))]
    {
        let _ = (cmd, limits);
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
