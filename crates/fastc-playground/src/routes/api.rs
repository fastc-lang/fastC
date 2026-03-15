use crate::compiler::{
    CheckRequest, CheckResponse, CompileErrorInfo, CompileRequest, CompileResponse, FormatRequest,
    FormatResponse, RunRequest, RunResponse,
};
use crate::executor::Executor;
use crate::server::AppState;
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::HeaderMap,
};
use std::net::SocketAddr;

fn simple_error(message: String) -> CompileErrorInfo {
    CompileErrorInfo {
        message,
        span: None,
        line: None,
        hint: None,
    }
}

/// POST /api/compile - Compile FastC code to C
pub async fn compile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CompileRequest>,
) -> Json<CompileResponse> {
    if let Err(message) = state.authorize(&headers) {
        return Json(CompileResponse {
            success: false,
            c_code: None,
            header: None,
            error: Some(simple_error(message)),
        });
    }

    if req.code.len() > state.config.max_code_bytes {
        return Json(CompileResponse {
            success: false,
            c_code: None,
            header: None,
            error: Some(simple_error(format!(
                "Code exceeds maximum size of {} bytes",
                state.config.max_code_bytes
            ))),
        });
    }

    Json(crate::compiler::compile(&req.code, req.emit_header))
}

/// POST /api/check - Type-check code without compiling
pub async fn check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CheckRequest>,
) -> Json<CheckResponse> {
    if let Err(message) = state.authorize(&headers) {
        return Json(CheckResponse {
            success: false,
            diagnostics: None,
            error: Some(simple_error(message)),
        });
    }

    if req.code.len() > state.config.max_code_bytes {
        return Json(CheckResponse {
            success: false,
            diagnostics: None,
            error: Some(simple_error(format!(
                "Code exceeds maximum size of {} bytes",
                state.config.max_code_bytes
            ))),
        });
    }

    Json(crate::compiler::check(&req.code))
}

/// POST /api/format - Format FastC code
pub async fn format(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FormatRequest>,
) -> Json<FormatResponse> {
    if let Err(message) = state.authorize(&headers) {
        return Json(FormatResponse {
            success: false,
            formatted: None,
            error: Some(simple_error(message)),
        });
    }

    if req.code.len() > state.config.max_code_bytes {
        return Json(FormatResponse {
            success: false,
            formatted: None,
            error: Some(simple_error(format!(
                "Code exceeds maximum size of {} bytes",
                state.config.max_code_bytes
            ))),
        });
    }

    Json(crate::compiler::format(&req.code))
}

/// POST /api/run - Compile and execute code
pub async fn run(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RunRequest>,
) -> Json<RunResponse> {
    if let Err(message) = state.authorize(&headers) {
        return Json(RunResponse {
            success: false,
            session_id: None,
            c_code: None,
            error: Some(simple_error(message)),
        });
    }

    if let Err(message) = state.validate_run_request(remote.ip(), req.code.len()) {
        return Json(RunResponse {
            success: false,
            session_id: None,
            c_code: None,
            error: Some(simple_error(message)),
        });
    }

    let Ok(run_slot) = state.try_acquire_run_slot() else {
        return Json(RunResponse {
            success: false,
            session_id: None,
            c_code: None,
            error: Some(simple_error(
                "Server is busy. Try again shortly.".to_string(),
            )),
        });
    };

    // First compile to check for errors and get C code
    let compile_result = crate::compiler::compile(&req.code, false);
    if !compile_result.success {
        return Json(RunResponse {
            success: false,
            session_id: None,
            c_code: None,
            error: compile_result.error,
        });
    }

    // Create a session for this execution
    let session_id = state.sessions.create();
    state.sessions.update_code(session_id, req.code.clone());

    // Create a broadcast channel for this session
    let tx = state.sessions.create_channel(session_id);

    // Start execution in background
    let code = req.code;
    let sessions = state.sessions.clone();
    let executor_limits = state.executor_limits.clone();
    tokio::spawn(async move {
        let _run_slot = run_slot;
        let executor = Executor::new(executor_limits);

        if let Err(e) = executor.run(session_id, &code, tx).await {
            tracing::error!("Execution failed: {}", e);
        }

        // Clean up the channel after execution completes
        sessions.remove_channel(session_id);
    });

    Json(RunResponse {
        success: true,
        session_id: Some(session_id.to_string()),
        c_code: compile_result.c_code,
        error: None,
    })
}
