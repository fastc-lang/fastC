use crate::compiler::{
    CheckRequest, CheckResponse, CompileRequest, CompileResponse, FormatRequest, FormatResponse,
    RunRequest, RunResponse,
};
use crate::executor::Executor;
use crate::server::AppState;
use axum::{extract::State, Json};

/// POST /api/compile - Compile FastC code to C
pub async fn compile(
    State(_state): State<AppState>,
    Json(req): Json<CompileRequest>,
) -> Json<CompileResponse> {
    Json(crate::compiler::compile(&req.code, req.emit_header))
}

/// POST /api/check - Type-check code without compiling
pub async fn check(
    State(_state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> Json<CheckResponse> {
    Json(crate::compiler::check(&req.code))
}

/// POST /api/format - Format FastC code
pub async fn format(
    State(_state): State<AppState>,
    Json(req): Json<FormatRequest>,
) -> Json<FormatResponse> {
    Json(crate::compiler::format(&req.code))
}

/// POST /api/run - Compile and execute code
pub async fn run(State(state): State<AppState>, Json(req): Json<RunRequest>) -> Json<RunResponse> {
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
    tokio::spawn(async move {
        let executor = Executor::new();

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
