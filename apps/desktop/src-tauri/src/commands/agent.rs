// Agent loop command — autonomous LLM tool-use cycle for UNI Assistant mode
use std::sync::Arc;
use sqlx::Row;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::AgentCancelTokens;
use crate::commands::budget::{catalog_id, check_budget, compute_cost_from_catalog, provider_from_base_url, write_cost_ledger};
use crate::commands::chat::{mcp_tools_to_openai, parse_tool_call_name, format_api_error};
use crate::models::db::AgentRun;
use crate::services::http_client::build_http_client;
use crate::services::llm_stream::{
    stream_llm_call, LlmStreamConfig,
    StreamDonePayload, StreamErrorPayload,
    StreamToolCallPayload, StreamToolResultPayload,
};
use crate::services::mcp_manager::McpManager;
use crate::services::memory_vector_store::MemoryVectorStore;
use crate::services::web_search;
use crate::services::web_content;
use crate::services::kb_vector_store::KbVectorStore;
use crate::services::kb_search::KbSearchConfig;
use crate::services::rag_context;
use crate::services::context_manager;
use crate::models::agent_trace::{AgentRunTrace, AgentStepTrace, LlmCallTrace, ToolCallTrace, is_builtin_tool};
use uni_common::safe_truncate_chars;
use uni_embedding::EmbeddingProvider;

type Pool = sqlx::SqlitePool;

async fn build_kb_embedding_provider(
    app: &AppHandle,
    emb_model: &str,
) -> Option<Box<dyn EmbeddingProvider>> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").ok()?;
    let api_key = match emb_model {
        "gemini" | "gemini-embedding" => store
            .get("embeddingGeminiKey")
            .and_then(|v| v.as_str().map(String::from))
            .filter(|s| !s.is_empty()),
        _ => store
            .get("embeddingOpenaiKey")
            .and_then(|v| v.as_str().map(String::from))
            .filter(|s| !s.is_empty()),
    }?;
    let model_id = match emb_model {
        "gemini" | "gemini-embedding" => "gemini",
        _ => "openai",
    };
    let client = build_http_client(app, None).await.ok()?;
    uni_embedding::create_embedding_provider(model_id, &api_key, client).ok()
}

const MAX_AGENT_ITERATIONS: i64 = 25;

// ─── Event payloads ──────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRunStartedPayload {
    run_id: String,
    chat_id: String,
    max_iterations: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentStepPayload {
    run_id: String,
    chat_id: String,
    iteration: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRunFinishedPayload {
    run_id: String,
    status: String,
    iterations: i64,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRunLimitPayload {
    run_id: String,
    iteration: i64,
    max_iterations: i64,
}

// ─── Commands ────────────────────────────────────────────────────

#[tauri::command]
pub async fn send_agent_message(
    app: AppHandle,
    pool: State<'_, Pool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    kb_vector_store: State<'_, Arc<Option<KbVectorStore>>>,
    cancel_tokens: State<'_, AgentCancelTokens>,
    chat_id: String,
    content: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
    system_prompt: Option<String>,
    parent_id: Option<String>,
    user_message_id: Option<String>,
    assistant_message_id: Option<String>,
    max_iterations: Option<i64>,
    auto_mode: Option<bool>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    supports_tool_use: Option<bool>,
) -> Result<String, String> {
    if content.is_empty() && user_message_id.is_none() {
        return Err("Empty message".to_string());
    }

    let run_id = uni_common::generate_id();
    let max_iter = max_iterations.unwrap_or(MAX_AGENT_ITERATIONS);
    let auto = auto_mode.unwrap_or(true);
    let now = uni_common::now_unix_secs();

    // Routing: optionally override model from rules or LLM
    let mut run_model = model.clone();
    let mut run_base_url = base_url.clone();
    let mut run_api_key = api_key.clone();
    let mut assigned_model: Option<String> = None;

    let store = tauri_plugin_store::StoreExt::store(&app, "settings.json").map_err(|e: tauri_plugin_store::Error| e.to_string())?;
    let routing_enabled: bool = store.get("routingEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let routing_strategy: String = store.get("routingStrategy").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "rules".to_string());

    if routing_enabled {
        let catalog_id_opt = if routing_strategy == "llm" {
            crate::commands::routing::llm_route_task(
                pool.inner(),
                &app,
                &chat_id,
                &run_api_key,
                run_base_url.as_deref().unwrap_or("https://openrouter.ai/api/v1"),
                &run_model,
                &content,
            ).await.ok().flatten()
        } else {
            let skill_name = crate::commands::skills::get_chat_skills_impl(pool.inner(), &chat_id)
                .await
                .ok()
                .and_then(|skills| skills.first().map(|s| s.name.clone()));
            let category = match skill_name.as_deref() {
                Some("Programmer") => "coding",
                Some("Writer") => "writing",
                Some("Analyst") => "analysis",
                _ => "general",
            };
            crate::commands::routing::get_model_for_category(pool.inner(), category).await
        };
        if let Some(ref cid) = catalog_id_opt {
            if let Ok((m, b, k)) = crate::commands::planner::resolve_catalog_id_to_credentials(pool.inner(), &app, cid).await {
                run_model = m;
                run_base_url = b;
                run_api_key = k;
                assigned_model = Some(cid.clone());
            }
        }
    }

    // Create agent_run record
    sqlx::query(
        "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at, assigned_model) VALUES (?, ?, 'running', 0, ?, ?, ?)"
    )
    .bind(&run_id).bind(&chat_id).bind(max_iter).bind(now).bind(&assigned_model)
    .execute(pool.inner()).await.map_err(|e| e.to_string())?;

    // Save user message to DB (with agent metadata)
    let user_msg_id = user_message_id.unwrap_or_else(|| uni_common::generate_id());
    sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'user', ?, ?, ?, '', 0, 0, 0.0, 0, 0, ?)"
    )
    .bind(&user_msg_id).bind(&chat_id).bind(&content).bind(&parent_id).bind(now).bind(&run_id)
    .execute(pool.inner()).await.map_err(|e| e.to_string())?;

    // Update active_child_map for branching support
    if let Some(ref pid) = parent_id {
        let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(pool.inner())
            .await
            .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|_| "{}".to_string());
        let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
        map.insert(pid.clone(), user_msg_id.clone());
        let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
        let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
            .bind(&new_map).bind(&chat_id)
            .execute(pool.inner()).await;
    }

    // Emit agent-run-started
    let _ = app.emit("agent-run-started", AgentRunStartedPayload {
        run_id: run_id.clone(),
        chat_id: chat_id.clone(),
        max_iterations: max_iter,
    });

    // Create and store cancel token
    let cancel_token = CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(run_id.clone(), cancel_token.clone());
    }

    // Clone everything needed for the spawned task
    let pool_clone = pool.inner().clone();
    let mcp_mgr = mcp_manager.inner().clone();
    let mem_store = memory_store.inner().clone();
    let kb_store = kb_vector_store.inner().clone();
    let app_clone = app.clone();
    let run_id_clone = run_id.clone();
    let assistant_msg_id = assistant_message_id.unwrap_or_else(|| uni_common::generate_id());
    let cancel_tokens_clone = cancel_tokens.inner().clone();

    // Spawn the agent loop (use routed model/credentials if routing was applied)
    tokio::spawn(async move {
        agent_loop(
            app_clone,
            pool_clone,
            mcp_mgr,
            mem_store,
            kb_store,
            chat_id,
            run_id_clone,
            run_model,
            run_base_url,
            run_api_key,
            system_prompt,
            user_msg_id,
            assistant_msg_id,
            max_iter,
            auto,
            cancel_token,
            cancel_tokens_clone,
            temperature,
            max_tokens,
            top_p,
            top_k,
            frequency_penalty,
            presence_penalty,
            supports_tool_use,
            None, // scope_agent_run_id
            None, // plan_id
            0,    // depth — orchestrator level
        ).await;
    });

    Ok(run_id)
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    pool: State<'_, Pool>,
    cancel_tokens: State<'_, AgentCancelTokens>,
    run_id: String,
) -> Result<(), String> {
    // Cancel the token
    {
        let tokens = cancel_tokens.read().await;
        if let Some(token) = tokens.get(&run_id) {
            token.cancel();
        }
    }

    // Update DB
    let _ = sqlx::query("UPDATE agent_runs SET status = 'cancelled' WHERE id = ? AND status IN ('running', 'paused')")
        .bind(&run_id)
        .execute(pool.inner()).await;

    // Cascade cancel to sub-agents
    let sub_runs: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM agent_runs WHERE parent_run_id = ? AND status = 'running'"
    )
    .bind(&run_id)
    .fetch_all(pool.inner()).await
    .unwrap_or_default();

    let now = uni_common::now_unix_secs();

    for sub_run_id in &sub_runs {
        {
            let tokens = cancel_tokens.read().await;
            if let Some(token) = tokens.get(sub_run_id) {
                token.cancel();
            }
        }
        let _ = sqlx::query("UPDATE agent_runs SET status = 'cancelled', finished_at = ? WHERE id = ?")
            .bind(now).bind(sub_run_id)
            .execute(pool.inner()).await;
    }

    let _ = app.emit("agent-run-finished", AgentRunFinishedPayload {
        run_id,
        status: "cancelled".to_string(),
        iterations: 0,
        error: None,
    });

    Ok(())
}

#[tauri::command]
pub async fn resume_agent_run(
    app: AppHandle,
    pool: State<'_, Pool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    kb_vector_store: State<'_, Arc<Option<KbVectorStore>>>,
    cancel_tokens: State<'_, AgentCancelTokens>,
    run_id: String,
) -> Result<(), String> {
    // Read agent_run from DB
    let row = sqlx::query("SELECT chat_id, iterations, max_iterations FROM agent_runs WHERE id = ? AND status = 'paused'")
        .bind(&run_id)
        .fetch_optional(pool.inner()).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Agent run not found or not paused".to_string())?;

    let chat_id: String = row.get("chat_id");
    let current_iterations: i64 = row.get("iterations");
    let max_iterations: i64 = row.get("max_iterations");

    // Read chat settings
    let chat_row = sqlx::query("SELECT model, system_prompt, provider_id FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_one(pool.inner()).await
        .map_err(|e| e.to_string())?;

    let model: String = chat_row.try_get("model").unwrap_or_default();
    let system_prompt: Option<String> = chat_row.try_get::<Option<String>, _>("system_prompt").ok().flatten().filter(|s| !s.is_empty());

    // Update status to running
    let _ = sqlx::query("UPDATE agent_runs SET status = 'running' WHERE id = ?")
        .bind(&run_id)
        .execute(pool.inner()).await;

    // Create new cancel token
    let cancel_token = CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(run_id.clone(), cancel_token.clone());
    }

    let pool_clone = pool.inner().clone();
    let mcp_mgr = mcp_manager.inner().clone();
    let mem_store = memory_store.inner().clone();
    let kb_store_clone = kb_vector_store.inner().clone();
    let cancel_tokens_clone = cancel_tokens.inner().clone();

    let _ = app.emit("agent-run-started", AgentRunStartedPayload {
        run_id: run_id.clone(),
        chat_id: chat_id.clone(),
        max_iterations,
    });

    let run_id_clone = run_id.clone();
    tokio::spawn(async move {
        agent_loop_resume(
            app,
            pool_clone,
            mcp_mgr,
            mem_store,
            kb_store_clone,
            chat_id,
            run_id_clone,
            model,
            system_prompt,
            current_iterations,
            max_iterations,
            cancel_token,
            cancel_tokens_clone,
        ).await;
    });

    Ok(())
}

#[tauri::command]
pub async fn get_agent_runs(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<Vec<AgentRun>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, status, iterations, max_iterations, started_at, finished_at, error, cost, assigned_model FROM agent_runs WHERE chat_id = ? ORDER BY started_at DESC"
    )
    .bind(&chat_id)
    .fetch_all(pool.inner()).await
    .map_err(|e| e.to_string())?;

    let runs = rows.iter().map(|row| AgentRun {
        id: row.get("id"),
        chat_id: row.get("chat_id"),
        status: row.get("status"),
        iterations: row.get("iterations"),
        max_iterations: row.get("max_iterations"),
        started_at: row.get("started_at"),
        finished_at: row.try_get("finished_at").ok(),
        error: row.try_get::<Option<String>, _>("error").ok().flatten(),
        cost: row.try_get("cost").ok(),
        assigned_model: row.try_get("assigned_model").ok(),
    }).collect();

    Ok(runs)
}

#[tauri::command]
pub async fn get_sub_agent_runs(
    pool: State<'_, Pool>,
    parent_run_id: String,
) -> Result<Vec<crate::services::sub_agent::SubAgentRunInfo>, String> {
    let rows = sqlx::query(
        "SELECT id, parent_run_id, status, spawn_config, iterations, max_iterations, cost, assigned_model FROM agent_runs WHERE parent_run_id = ? ORDER BY started_at DESC"
    )
    .bind(&parent_run_id)
    .fetch_all(pool.inner()).await
    .map_err(|e| e.to_string())?;

    let runs = rows.iter().map(|row| {
        let spawn_config: String = row.try_get::<String, _>("spawn_config").unwrap_or_default();
        let goal = serde_json::from_str::<serde_json::Value>(&spawn_config)
            .ok()
            .and_then(|v| v.get("goal").and_then(|g| g.as_str()).map(String::from))
            .unwrap_or_default();
        crate::services::sub_agent::SubAgentRunInfo {
            id: row.get("id"),
            parent_run_id: row.try_get::<Option<String>, _>("parent_run_id").ok().flatten(),
            status: row.get("status"),
            goal,
            iterations: row.get("iterations"),
            max_iterations: row.get("max_iterations"),
            cost: row.try_get("cost").ok(),
            assigned_model: row.try_get::<Option<String>, _>("assigned_model").ok().flatten(),
        }
    }).collect();

    Ok(runs)
}

#[tauri::command]
pub async fn get_agent_run_trace(
    pool: State<'_, Pool>,
    run_id: String,
) -> Result<Option<AgentRunTrace>, String> {
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT trace FROM agent_runs WHERE id = ?"
    )
    .bind(&run_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    match row {
        Some((Some(trace_json),)) => {
            let trace: AgentRunTrace = serde_json::from_str(&trace_json)
                .map_err(|e| e.to_string())?;
            Ok(Some(trace))
        }
        _ => Ok(None),
    }
}

// ─── Agent Loop ──────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) async fn agent_loop(
    app: AppHandle,
    pool: Pool,
    mcp_mgr: Arc<McpManager>,
    mem_store: Arc<Option<MemoryVectorStore>>,
    kb_store: Arc<Option<KbVectorStore>>,
    chat_id: String,
    run_id: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
    system_prompt: Option<String>,
    last_user_msg_id: String,
    assistant_msg_id: String,
    max_iterations: i64,
    auto_mode: bool,
    cancel_token: CancellationToken,
    cancel_tokens: AgentCancelTokens,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    supports_tool_use: Option<bool>,
    scope_agent_run_id: Option<String>,
    plan_id: Option<String>,
    depth: i64,
) {
    let skip_branching = scope_agent_run_id.is_some();

    // Spawn tracker: prevents orchestrator from endlessly spawning failing sub-agents
    let mut spawn_tracker = if depth == 0 {
        Some(crate::services::spawn_tracker::SpawnTracker::new(10, 2))
    } else {
        None
    };

    // Resolve project context for this chat
    let project_id: Option<String> = sqlx::query_scalar(
        "SELECT p.id FROM projects p JOIN chats c ON c.project_id = p.id WHERE c.id = ?"
    )
    .bind(&chat_id)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    // Inject <project> block into system prompt if chat belongs to a project
    let system_prompt = if let Some(ref pid) = project_id {
        let project_row = sqlx::query("SELECT name, goal FROM projects WHERE id = ?")
            .bind(pid)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
        if let Some(row) = project_row {
            let pname: String = row.get("name");
            let pgoal: Option<String> = row.try_get::<Option<String>, _>("goal").ok().flatten();
            if let Some(ref goal) = pgoal {
                if !goal.is_empty() {
                    let project_block = format!("\n\n<project name=\"{}\">\n{}\n</project>", pname, goal);
                    Some(system_prompt.unwrap_or_default() + &project_block)
                } else {
                    system_prompt
                }
            } else {
                system_prompt
            }
        } else {
            system_prompt
        }
    } else {
        system_prompt
    };

    // Resolve KB context for this chat
    let kb_id: Option<String> = sqlx::query_scalar("SELECT kb_id FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None)
        .flatten();

    // Build embedding provider for KB (if attached)
    let kb_provider: Option<Box<dyn EmbeddingProvider>> =
        if let Some(ref kb_id_val) = kb_id {
            let emb_model: String =
                sqlx::query_scalar("SELECT embedding_model FROM knowledge_bases WHERE id = ?")
                    .bind(kb_id_val)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "openai".to_string());
            build_kb_embedding_provider(&app, &emb_model).await
        } else {
            None
        };

    // Build query processing config for KB search (if KB attached)
    let kb_query_config: Option<crate::services::query_processor::QueryProcessingConfig> = if kb_id.is_some() {
        let (qr, qd, qv) = if let Some(ref kb_id_val) = kb_id {
            let row = sqlx::query("SELECT query_rewriting_enabled, query_decomposition_enabled, query_max_variants, reranker_type, reranker_overfetch_factor FROM knowledge_bases WHERE id = ?")
                .bind(kb_id_val)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
            match row {
                Some(r) => {
                    use sqlx::Row;
                    (
                        r.try_get::<bool, _>("query_rewriting_enabled").unwrap_or(true),
                        r.try_get::<bool, _>("query_decomposition_enabled").unwrap_or(false),
                        r.try_get::<i64, _>("query_max_variants").unwrap_or(3),
                    )
                }
                None => (true, false, 3),
            }
        } else {
            (true, false, 3)
        };
        Some(crate::commands::knowledge_base::build_query_config_from_kb(
            qr, qd, qv,
            Some(model.clone()),
            Some(api_key.clone()),
            base_url.clone(),
        ))
    } else {
        None
    };

    // Build reranker config for KB search (if KB attached)
    let kb_reranker_config: Option<crate::services::kb_search_orchestrator::RerankerConfig> = if kb_id.is_some() {
        let (rt, rof) = if let Some(ref kb_id_val) = kb_id {
            let row = sqlx::query("SELECT reranker_type, reranker_overfetch_factor FROM knowledge_bases WHERE id = ?")
                .bind(kb_id_val)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();
            match row {
                Some(r) => {
                    use sqlx::Row;
                    (
                        r.try_get::<String, _>("reranker_type").unwrap_or("none".to_string()),
                        r.try_get::<i64, _>("reranker_overfetch_factor").unwrap_or(4),
                    )
                }
                None => ("none".to_string(), 4),
            }
        } else {
            ("none".to_string(), 4)
        };
        let (ck, jk) = {
            use tauri_plugin_store::StoreExt;
            let store_settings = app.store("settings.json").ok();
            let ck = store_settings.as_ref().and_then(|s| s.get("rerankerCohereKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()));
            let jk = store_settings.as_ref().and_then(|s| s.get("rerankerJinaKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()));
            (ck, jk)
        };
        Some(crate::commands::knowledge_base::build_reranker_config_from_kb(&rt, rof, &ck, &jk))
    } else {
        None
    };

    // Inject RAG system instruction into system prompt if KB attached
    let system_prompt = if let Some(ref kb_id_val) = kb_id {
        let kb_row = sqlx::query("SELECT system_prompt FROM knowledge_bases WHERE id = ?")
            .bind(kb_id_val)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
        if let Some(row) = kb_row {
            let kb_sys: String = row.get("system_prompt");
            let instruction = rag_context::build_rag_system_instruction(&kb_sys);
            Some(format!("{}\n\n{}", instruction, system_prompt.unwrap_or_default()))
        } else {
            system_prompt
        }
    } else {
        system_prompt
    };

    // Read web search settings from store
    let (web_search_enabled, web_search_provider, ws_tavily_key, ws_brave_key) = {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store("settings.json") {
            let enabled = store.get("webSearchEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let provider = store.get("webSearchProvider").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "duckduckgo".to_string());
            let tavily = store.get("tavilyApiKey").and_then(|v| v.as_str().map(String::from));
            let brave = store.get("braveApiKey").and_then(|v| v.as_str().map(String::from));
            (enabled, provider, tavily, brave)
        } else {
            (false, "duckduckgo".to_string(), None, None)
        }
    };

    // Append web search hint to system prompt
    let system_prompt = if web_search_enabled {
        let hint = "\n\nYou have web_search and web_read tools for internet access. ALWAYS use web_search directly when you need current information, news, or facts — do not delegate this to sub-agents.";
        Some(system_prompt.unwrap_or_default() + hint)
    } else {
        system_prompt
    };

    // Append KB search hint to system prompt
    let system_prompt = if kb_id.is_some() {
        let hint = "\n\nYou have access to a knowledge base via the kb_search tool. Use it to look up relevant information before answering questions that might be covered by the knowledge base documents. Always cite the source document when using information from the knowledge base.";
        Some(system_prompt.unwrap_or_default() + hint)
    } else {
        system_prompt
    };

    // Append orchestrator guidance (depth 0 only) — prefer direct tools over sub-agents for simple tasks
    let system_prompt = if depth == 0 {
        let orchestrator_hint = "\n\nTool usage strategy:\n- For simple lookups (search memory, search KB, search web): use the tool directly. Do NOT spawn a sub-agent for a single search.\n- For reading a webpage: use web_read directly.\n- For saving a fact: use memory_save directly.\n- For writing a document: use workspace_write directly.\n- Spawn a sub-agent ONLY when:\n  a) The task requires multiple sequential steps (research \u{2192} analyze \u{2192} write)\n  b) Multiple independent tasks can run in parallel\n  c) The task needs a specialized skill or different model\n- After spawning sub-agents, WAIT before checking status. Sub-agents need at least 30 seconds to complete meaningful work. Spawn all needed sub-agents first, do other work, then check their status.";
        Some(system_prompt.unwrap_or_default() + orchestrator_hint)
    } else {
        system_prompt
    };

    // Inject current date so the agent knows today's date (for web searches, etc.)
    let system_prompt = {
        let date_str = chrono::Local::now().format("%Y-%m-%d %A").to_string();
        let date_line = format!("\n\nCurrent date: {}.", date_str);
        Some(system_prompt.unwrap_or_default() + &date_line)
    };

    // Inject tool routing hint — helps the model choose the right search tool
    let system_prompt = {
        let routing = build_tool_routing_hint(kb_id.is_some(), web_search_enabled, true);
        if routing.is_empty() { system_prompt } else { Some(system_prompt.unwrap_or_default() + &routing) }
    };

    let base = base_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
    let url = uni_llm::build_llm_url(base.trim_end_matches('/'));
    let headers = uni_llm::build_llm_headers(&api_key);

    let client = match build_http_client(&app, None).await {
        Ok(c) => c,
        Err(e) => {
            finish_run(&app, &pool, &run_id, &chat_id, "failed", 0, Some(e), None).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }
    };

    // Build MCP tools
    let tool_use_supported = supports_tool_use.unwrap_or(true);
    let (mut tools_json, tool_name_map) = if tool_use_supported {
        let mcp_tools = mcp_mgr.get_all_tools().await;
        if !mcp_tools.is_empty() {
            let (openai_tools, name_map) = mcp_tools_to_openai(&mcp_tools);
            (Some(serde_json::Value::Array(openai_tools)), name_map)
        } else {
            (None, std::collections::HashMap::new())
        }
    } else {
        (None, std::collections::HashMap::new())
    };

    // Inject built-in memory + workspace tools (+ orchestrator tools at depth 0)
    if tool_use_supported {
        let mut builtin_tools = memory_tool_definitions();
        builtin_tools.extend(workspace_tool_definitions());
        if depth == 0 {
            builtin_tools.extend(orchestrator_tool_definitions());
        }
        if web_search_enabled {
            builtin_tools.extend(web_search_tool_definitions());
        }
        if kb_id.is_some() {
            builtin_tools.extend(kb_search_tool_definitions());
        }
        if let Some(ref mut tools) = tools_json {
            if let Some(arr) = tools.as_array_mut() {
                arr.extend(builtin_tools);
            }
        } else {
            tools_json = Some(serde_json::Value::Array(builtin_tools));
        }
    }

    // Load skills attached to this chat
    let chat_skills = crate::commands::skills::get_chat_skills_impl(&pool, &chat_id)
        .await
        .unwrap_or_default();
    let skill_content = if chat_skills.is_empty() {
        None
    } else {
        Some(
            chat_skills
                .iter()
                .map(|s| format!("<skill name=\"{}\">\n{}\n</skill>", s.name, s.content))
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    };

    let mut iteration: i64 = 0;
    let mut last_parent_id = last_user_msg_id;
    let mut accumulated_content = String::new();
    let mut run_trace = AgentRunTrace::new(run_id.clone(), chat_id.clone(), model.clone(), chrono::Utc::now().timestamp());

    loop {
        // Check cancellation
        if cancel_token.is_cancelled() {
            run_trace.finish("cancelled", None);
            finish_run(&app, &pool, &run_id, &chat_id, "cancelled", iteration, None, Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }

        // Check budget before this LLM call
        if let Err(_) = check_budget(&pool, &app, &chat_id, plan_id.as_deref()).await {
            run_trace.finish("paused_budget", None);
            finish_run(&app, &pool, &run_id, &chat_id, "paused_budget", iteration, None, Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }

        // Check iteration limit
        if iteration >= max_iterations {
            let _ = app.emit("agent-run-limit", AgentRunLimitPayload {
                run_id: run_id.clone(),
                iteration,
                max_iterations,
            });
            run_trace.finish("completed", Some("max_steps".to_string()));
            finish_run(&app, &pool, &run_id, &chat_id, "completed", iteration, Some("Max iterations reached".to_string()), Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }

        iteration += 1;
        let step_start = std::time::Instant::now();
        let step_started_at = chrono::Utc::now().timestamp();

        // Emit step start
        log::info!("Agent iteration {} starting LLM call for run {}", iteration, run_id);
        let _ = app.emit("agent-step-start", AgentStepPayload {
            run_id: run_id.clone(),
            chat_id: chat_id.clone(),
            iteration,
        });

        // Build messages from DB
        let messages = match build_agent_messages(&pool, &app, &chat_id, &system_prompt, mem_store.as_ref().as_ref(), &skill_content, scope_agent_run_id.as_deref(), project_id.as_deref(), kb_id.as_deref(), kb_store.as_ref().as_ref(), kb_provider.as_deref(), &client, kb_query_config.as_ref(), kb_reranker_config.as_ref()).await {
            Ok(m) => m,
            Err(e) => {
                let _ = app.emit("agent-stream-error", StreamErrorPayload { error: e.clone() });
                run_trace.finish("failed", None);
                finish_run(&app, &pool, &run_id, &chat_id, "failed", iteration, Some(e), Some(&run_trace)).await;
                cleanup_cancel_token(&cancel_tokens, &run_id).await;
                return;
            }
        };

        // Trim messages to fit context window
        let trim_config = context_manager::TrimConfig {
            model_context_limit: context_manager::get_model_context_limit(&model),
            ..Default::default()
        };
        let trim_result = context_manager::trim_messages(&messages, &trim_config);
        if trim_result.was_trimmed {
            eprintln!(
                "[Agent] Context trimmed: {} → {} messages, {} → {} tokens ({:.0}% of {})",
                trim_result.original_count, trim_result.trimmed_count,
                trim_result.original_tokens, trim_result.final_tokens,
                trim_result.usage_ratio * 100.0, trim_config.model_context_limit
            );
        }
        let _ = app.emit("agent-context-usage", serde_json::json!({
            "runId": run_id,
            "usageRatio": trim_result.usage_ratio,
            "totalTokens": trim_result.final_tokens,
            "modelLimit": trim_config.model_context_limit,
            "wasTrimmed": trim_result.was_trimmed,
        }));
        let messages = trim_result.messages;

        // Build request body
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        if let Some(t) = temperature { body["temperature"] = serde_json::json!(t); }
        if let Some(m) = max_tokens { body["max_tokens"] = serde_json::json!(m); }
        if let Some(p) = top_p { body["top_p"] = serde_json::json!(p); }
        if let Some(k) = top_k { body["top_k"] = serde_json::json!(k); }
        if let Some(fp) = frequency_penalty { body["frequency_penalty"] = serde_json::json!(fp); }
        if let Some(pp) = presence_penalty { body["presence_penalty"] = serde_json::json!(pp); }
        if let Some(ref tools) = tools_json {
            body["tools"] = tools.clone();
        }

        // Stream LLM call
        let config = LlmStreamConfig {
            client: &client,
            url: &url,
            headers: headers.clone(),
            body: &body,
            cancel_token: &cancel_token,
            event_prefix: "agent-stream",
            app: &app,
        };

        let result = stream_llm_call(&config).await;

        match result {
            Err(e) if e == "__cancelled__" => {
                // Emit done with accumulated content
                let _ = app.emit("agent-stream-done", StreamDonePayload {
                    full_content: accumulated_content.clone(),
                });
                run_trace.finish("cancelled", Some("cancelled".to_string()));
                finish_run(&app, &pool, &run_id, &chat_id, "cancelled", iteration, None, Some(&run_trace)).await;
                cleanup_cancel_token(&cancel_tokens, &run_id).await;
                return;
            }
            Err(e) => {
                let user_error = format_api_error(
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR, &e, &model
                );
                let _ = app.emit("agent-stream-error", StreamErrorPayload { error: user_error.clone() });
                run_trace.finish("failed", None);
                finish_run(&app, &pool, &run_id, &chat_id, "failed", iteration, Some(user_error), Some(&run_trace)).await;
                cleanup_cancel_token(&cancel_tokens, &run_id).await;
                return;
            }
            Ok(stream_result) => {
                let llm_duration_ms = step_start.elapsed().as_millis() as u64;
                log::info!("Agent iteration {} LLM call complete, tool_calls: {}, content_len: {}",
                    iteration, stream_result.tool_calls.len(), stream_result.content.len());
                accumulated_content.push_str(&stream_result.content);

                // Save assistant message to DB
                let asst_msg_id = if iteration == 1 {
                    assistant_msg_id.clone()
                } else {
                    uni_common::generate_id()
                };
                let msg_now = uni_common::now_unix_secs();

                let pt = stream_result.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
                let ct = stream_result.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
                let provider = provider_from_base_url(&base);
                let cat_id = catalog_id(provider, &model);
                let cost = compute_cost_from_catalog(&pool, &cat_id, pt, ct).await;

                let llm_trace = LlmCallTrace {
                    input_tokens: pt as usize,
                    output_tokens: ct as usize,
                    cost,
                    duration_ms: llm_duration_ms,
                    had_tool_calls: !stream_result.tool_calls.is_empty(),
                    response_preview: safe_truncate_chars(&stream_result.content, 200),
                };
                let mut step_tool_traces: Vec<ToolCallTrace> = Vec::new();

                let _ = sqlx::query(
                    "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'assistant', ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)"
                )
                .bind(&asst_msg_id).bind(&chat_id).bind(&stream_result.content)
                .bind(&last_parent_id).bind(msg_now).bind(&model)
                .bind(pt as i64)
                .bind(ct as i64)
                .bind(cost)
                .bind(iteration).bind(&run_id)
                .execute(&pool).await;

                let _ = write_cost_ledger(
                    &pool,
                    &chat_id,
                    Some(&run_id),
                    plan_id.as_deref(),
                    &model,
                    provider,
                    pt,
                    ct,
                    cost,
                    if plan_id.is_some() { "plan_task" } else { "agent" },
                ).await;

                let run_cost: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE agent_run_id = ?")
                    .bind(&run_id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|row| row.try_get::<f64, _>("total").ok())
                    .unwrap_or(0.0);
                let _ = sqlx::query("UPDATE agent_runs SET cost = ? WHERE id = ?")
                    .bind(run_cost).bind(&run_id)
                    .execute(&pool).await;

                // Update active_child_map (skip for parallel plan tasks to avoid races)
                if !skip_branching {
                    let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
                        .bind(&chat_id)
                        .fetch_one(&pool)
                        .await
                        .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
                        .unwrap_or_else(|_| "{}".to_string());
                    let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
                    map.insert(last_parent_id.clone(), asst_msg_id.clone());
                    let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
                    let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
                        .bind(&new_map).bind(&chat_id)
                        .execute(&pool).await;
                }

                // No tool calls → agent is done
                if stream_result.tool_calls.is_empty() {
                    let step_trace = AgentStepTrace {
                        step: iteration as usize,
                        started_at: step_started_at,
                        duration_ms: step_start.elapsed().as_millis() as u64,
                        llm_call: llm_trace,
                        tool_calls: step_tool_traces,
                    };
                    let _ = app.emit("agent-step-trace", &step_trace);
                    run_trace.add_step(step_trace);
                    run_trace.finish("completed", Some("natural".to_string()));
                    // Emit step-done for the final iteration so frontend reloads DB messages
                    let _ = app.emit("agent-step-done", AgentStepPayload {
                        run_id: run_id.clone(),
                        chat_id: chat_id.clone(),
                        iteration,
                    });
                    let _ = app.emit("agent-stream-done", StreamDonePayload {
                        full_content: accumulated_content,
                    });
                    finish_run(&app, &pool, &run_id, &chat_id, "completed", iteration, None, Some(&run_trace)).await;
                    cleanup_cancel_token(&cancel_tokens, &run_id).await;
                    // Auto-extract memories in background
                    let ext_app = app.clone();
                    let ext_pool = pool.clone();
                    let ext_mem = mem_store.clone();
                    let ext_chat = chat_id.clone();
                    let ext_model = model.clone();
                    let ext_key = api_key.clone();
                    let ext_base = base.clone();
                    let ext_pid = project_id.clone();
                    tokio::spawn(async move {
                        extract_memories_from_run(&ext_app, &ext_pool, &ext_mem, &ext_chat, &ext_model, &ext_key, &ext_base, ext_pid.as_deref()).await;
                    });
                    return;
                }

                // Execute tool calls
                let mut tool_parent_id = asst_msg_id.clone();
                for tool_call in &stream_result.tool_calls {
                    if cancel_token.is_cancelled() {
                        run_trace.finish("cancelled", Some("cancelled".to_string()));
                        finish_run(&app, &pool, &run_id, &chat_id, "cancelled", iteration, None, Some(&run_trace)).await;
                        cleanup_cancel_token(&cancel_tokens, &run_id).await;
                        return;
                    }

                    let (server_id, tool_name) = parse_tool_call_name(&tool_call.name, &tool_name_map);

                    let _ = app.emit("agent-tool-call", StreamToolCallPayload {
                        tool_call_id: tool_call.id.clone(),
                        server_id: server_id.clone(),
                        tool_name: tool_name.clone(),
                        arguments: tool_call.arguments.clone(),
                    });

                    let args: serde_json::Value =
                        serde_json::from_str(&tool_call.arguments).unwrap_or(serde_json::json!({}));

                    let tool_start = std::time::Instant::now();

                    // Intercept built-in tools (memory + workspace + orchestrator)
                    let (result_text, is_error) = if tool_call.name == "memory_save" || tool_call.name == "memory_search" {
                        handle_memory_tool(&app, &pool, mem_store.as_ref().as_ref(), &tool_call.name, &args, Some(&chat_id), project_id.as_deref()).await
                    } else if tool_call.name == "workspace_write" || tool_call.name == "workspace_read" || tool_call.name == "workspace_list" {
                        let res = handle_workspace_tool(&pool, &chat_id, project_id.as_deref(), &run_id, &tool_call.name, &args).await;
                        if tool_call.name == "workspace_write" && !res.1 {
                            let _ = app.emit("workspace-changed", serde_json::json!({
                                "chatId": &chat_id,
                                "projectId": project_id.as_deref()
                            }));
                        }
                        res
                    } else if tool_call.name == "spawn_agent" || tool_call.name == "check_agent" || tool_call.name == "get_agent_result" || tool_call.name == "cancel_agent" {
                        // Delegation loop prevention: check spawn limits before spawning
                        if tool_call.name == "spawn_agent" {
                            if let Some(ref tracker) = spawn_tracker {
                                if let Some(warning) = tracker.check_limits() {
                                    let msg = crate::services::spawn_tracker::SpawnTracker::format_warning(&warning);
                                    (msg, true)
                                } else {
                                    let goal = args.get("goal").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let res = handle_orchestrator_tool(
                                        app.clone(), pool.clone(), mcp_mgr.clone(), mem_store.clone(), kb_store.clone(), cancel_tokens.clone(),
                                        chat_id.clone(), run_id.clone(), api_key.clone(), base.clone(),
                                        model.clone(), depth, &tool_call.name, &args,
                                    ).await;
                                    if !res.1 {
                                        if let Some(ref mut tracker) = spawn_tracker {
                                            tracker.record_spawn(&goal, iteration);
                                        }
                                    }
                                    res
                                }
                            } else {
                                handle_orchestrator_tool(
                                    app.clone(), pool.clone(), mcp_mgr.clone(), mem_store.clone(), kb_store.clone(), cancel_tokens.clone(),
                                    chat_id.clone(), run_id.clone(), api_key.clone(), base.clone(),
                                    model.clone(), depth, &tool_call.name, &args,
                                ).await
                            }
                        } else if tool_call.name == "get_agent_result" {
                            let res = handle_orchestrator_tool(
                                app.clone(), pool.clone(), mcp_mgr.clone(), mem_store.clone(), kb_store.clone(), cancel_tokens.clone(),
                                chat_id.clone(), run_id.clone(), api_key.clone(), base.clone(),
                                model.clone(), depth, &tool_call.name, &args,
                            ).await;
                            // Track sub-agent result status for loop detection
                            if !res.1 {
                                if let Some(ref mut tracker) = spawn_tracker {
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&res.0) {
                                        if let (Some(status), Some(_)) = (
                                            parsed.get("status").and_then(|v| v.as_str()),
                                            parsed.get("agent_run_id"),
                                        ) {
                                            // Find goal from spawn_config in DB or use run_id to match
                                            // For simplicity, we track by looking up the spawn record via DB
                                            let target = args.get("agent_run_id").and_then(|v| v.as_str()).unwrap_or("");
                                            if let Ok(Some(row)) = sqlx::query(
                                                "SELECT spawn_config FROM agent_runs WHERE id = ?"
                                            ).bind(target).fetch_optional(&pool).await {
                                                let config_json: Option<String> = row.try_get("spawn_config").ok();
                                                if let Some(config_str) = config_json {
                                                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                                                        if let Some(goal) = config.get("goal").and_then(|v| v.as_str()) {
                                                            tracker.record_result(goal, status);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            res
                        } else {
                            handle_orchestrator_tool(
                                app.clone(), pool.clone(), mcp_mgr.clone(), mem_store.clone(), kb_store.clone(), cancel_tokens.clone(),
                                chat_id.clone(), run_id.clone(), api_key.clone(), base.clone(),
                                model.clone(), depth, &tool_call.name, &args,
                            ).await
                        }
                    } else if tool_call.name == "web_search" || tool_call.name == "web_read" {
                        handle_web_search_tool(&app, &tool_call.name, &args, &web_search_provider, ws_tavily_key.as_deref(), ws_brave_key.as_deref()).await
                    } else if tool_call.name == "kb_search" {
                        if let Some(ref kb_id_val) = kb_id {
                            handle_kb_search_tool(&pool, kb_store.as_ref().as_ref(), kb_id_val, &args, kb_provider.as_deref(), &client, kb_query_config.as_ref(), kb_reranker_config.as_ref()).await
                        } else {
                            ("Knowledge base not attached to this chat.".to_string(), true)
                        }
                    } else {
                        let result = mcp_mgr.call_tool(&server_id, &tool_name, args).await;
                        match result {
                            Ok(r) => {
                                let text = r.content.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");
                                (text, r.is_error)
                            }
                            Err(e) => (e.to_string(), true),
                        }
                    };

                    let tool_duration_ms = tool_start.elapsed().as_millis() as u64;
                    step_tool_traces.push(ToolCallTrace {
                        tool_name: tool_call.name.clone(),
                        arguments_preview: safe_truncate_chars(&tool_call.arguments, 200),
                        result_preview: safe_truncate_chars(&result_text, 200),
                        duration_ms: tool_duration_ms,
                        is_error,
                        is_builtin: is_builtin_tool(&tool_call.name),
                    });

                    let _ = app.emit("agent-tool-result", StreamToolResultPayload {
                        tool_call_id: tool_call.id.clone(),
                        result: result_text.clone(),
                        is_error,
                    });

                    // Save tool result as message in DB
                    let tool_msg_id = uni_common::generate_id();
                    let tool_content = serde_json::json!({
                        "tool_call_id": tool_call.id,
                        "tool_name": tool_name,
                        "server_id": server_id,
                        "arguments": tool_call.arguments,
                        "result": result_text,
                        "is_error": is_error
                    }).to_string();
                    let tool_now = uni_common::now_unix_secs();

                    let _ = sqlx::query(
                        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'tool', ?, ?, ?, '', 0, 0, 0.0, 0, ?, ?)"
                    )
                    .bind(&tool_msg_id).bind(&chat_id).bind(&tool_content)
                    .bind(&tool_parent_id).bind(tool_now)
                    .bind(iteration).bind(&run_id)
                    .execute(&pool).await;

                    // Update active_child_map (skip for parallel plan tasks to avoid races)
                    if !skip_branching {
                        let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
                            .bind(&chat_id)
                            .fetch_one(&pool)
                            .await
                            .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
                            .unwrap_or_else(|_| "{}".to_string());
                        let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
                        map.insert(tool_parent_id.clone(), tool_msg_id.clone());
                        let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
                        let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
                            .bind(&new_map).bind(&chat_id)
                            .execute(&pool).await;
                    }

                    tool_parent_id = tool_msg_id;
                }

                // Build and emit step trace
                let step_trace = AgentStepTrace {
                    step: iteration as usize,
                    started_at: step_started_at,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    llm_call: llm_trace,
                    tool_calls: step_tool_traces,
                };
                let _ = app.emit("agent-step-trace", &step_trace);
                run_trace.add_step(step_trace);

                // Update last_parent_id for next iteration
                last_parent_id = tool_parent_id;

                // Step-by-step mode: pause
                if !auto_mode {
                    let _ = app.emit("agent-step-pause", AgentStepPayload {
                        run_id: run_id.clone(),
                        chat_id: chat_id.clone(),
                        iteration,
                    });
                    // Save trace so far for resume to pick up
                    let trace_json = serde_json::to_string(&run_trace).unwrap_or_default();
                    let _ = sqlx::query("UPDATE agent_runs SET status = 'paused', iterations = ?, trace = ? WHERE id = ?")
                        .bind(iteration).bind(&trace_json).bind(&run_id)
                        .execute(&pool).await;
                    // Don't cleanup cancel token — resume will use it
                    return;
                }

                // Emit step done, continue loop
                let _ = app.emit("agent-step-done", AgentStepPayload {
                    run_id: run_id.clone(),
                    chat_id: chat_id.clone(),
                    iteration,
                });

                // Update iteration count in DB
                let _ = sqlx::query("UPDATE agent_runs SET iterations = ? WHERE id = ?")
                    .bind(iteration).bind(&run_id)
                    .execute(&pool).await;
            }
        }
    }
}

/// Resume a paused agent loop (step-by-step mode)
#[allow(clippy::too_many_arguments)]
async fn agent_loop_resume(
    app: AppHandle,
    pool: Pool,
    mcp_mgr: Arc<McpManager>,
    mem_store: Arc<Option<MemoryVectorStore>>,
    kb_store: Arc<Option<KbVectorStore>>,
    chat_id: String,
    run_id: String,
    model: String,
    system_prompt: Option<String>,
    current_iterations: i64,
    max_iterations: i64,
    cancel_token: CancellationToken,
    cancel_tokens: AgentCancelTokens,
) {
    // Resolve project context for this chat
    let project_id: Option<String> = sqlx::query_scalar(
        "SELECT p.id FROM projects p JOIN chats c ON c.project_id = p.id WHERE c.id = ?"
    )
    .bind(&chat_id)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    // Resolve KB context for this chat
    let kb_id: Option<String> = sqlx::query_scalar("SELECT kb_id FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None)
        .flatten();

    // Build embedding provider for KB (if attached)
    let kb_provider: Option<Box<dyn EmbeddingProvider>> =
        if let Some(ref kb_id_val) = kb_id {
            let emb_model: String =
                sqlx::query_scalar("SELECT embedding_model FROM knowledge_bases WHERE id = ?")
                    .bind(kb_id_val)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "openai".to_string());
            build_kb_embedding_provider(&app, &emb_model).await
        } else {
            None
        };

    // Build query processing config for KB search (will be populated after api_key is resolved)
    // Placeholder — actual config built after credentials are available (line ~1160)
    let kb_query_config_fields: Option<(bool, bool, i64)> = if let Some(ref kb_id_val) = kb_id {
        let row = sqlx::query("SELECT query_rewriting_enabled, query_decomposition_enabled, query_max_variants FROM knowledge_bases WHERE id = ?")
            .bind(kb_id_val)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
        match row {
            Some(r) => {
                use sqlx::Row;
                Some((
                    r.try_get::<bool, _>("query_rewriting_enabled").unwrap_or(true),
                    r.try_get::<bool, _>("query_decomposition_enabled").unwrap_or(false),
                    r.try_get::<i64, _>("query_max_variants").unwrap_or(3),
                ))
            }
            None => None,
        }
    } else {
        None
    };

    // Build reranker config for KB search (if KB attached)
    let kb_reranker_config: Option<crate::services::kb_search_orchestrator::RerankerConfig> = if let Some(ref kb_id_val) = kb_id {
        let row = sqlx::query("SELECT reranker_type, reranker_overfetch_factor FROM knowledge_bases WHERE id = ?")
            .bind(kb_id_val)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
        match row {
            Some(r) => {
                use sqlx::Row;
                let rt: String = r.try_get("reranker_type").unwrap_or("none".to_string());
                let rof: i64 = r.try_get("reranker_overfetch_factor").unwrap_or(4);
                let (ck, jk) = {
                    use tauri_plugin_store::StoreExt;
                    let store_settings = app.store("settings.json").ok();
                    let ck = store_settings.as_ref().and_then(|s| s.get("rerankerCohereKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()));
                    let jk = store_settings.as_ref().and_then(|s| s.get("rerankerJinaKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()));
                    (ck, jk)
                };
                Some(crate::commands::knowledge_base::build_reranker_config_from_kb(&rt, rof, &ck, &jk))
            }
            None => None,
        }
    } else {
        None
    };

    // Read web search settings from store
    let (web_search_enabled, web_search_provider, ws_tavily_key, ws_brave_key) = {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store("settings.json") {
            let enabled = store.get("webSearchEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let provider = store.get("webSearchProvider").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "duckduckgo".to_string());
            let tavily = store.get("tavilyApiKey").and_then(|v| v.as_str().map(String::from));
            let brave = store.get("braveApiKey").and_then(|v| v.as_str().map(String::from));
            (enabled, provider, tavily, brave)
        } else {
            (false, "duckduckgo".to_string(), None, None)
        }
    };

    // Append web search hint to system prompt
    let system_prompt = if web_search_enabled {
        let hint = "\n\nYou have web_search and web_read tools for internet access. ALWAYS use web_search directly when you need current information, news, or facts — do not delegate this to sub-agents.";
        Some(system_prompt.unwrap_or_default() + hint)
    } else {
        system_prompt
    };

    // Append KB search hint to system prompt
    let system_prompt = if kb_id.is_some() {
        let hint = "\n\nYou have access to a knowledge base via the kb_search tool. Use it to look up relevant information before answering questions that might be covered by the knowledge base documents. Always cite the source document when using information from the knowledge base.";
        Some(system_prompt.unwrap_or_default() + hint)
    } else {
        system_prompt
    };

    // Append orchestrator guidance — resume is always depth 0 (sub-agents run in auto mode and never pause)
    let system_prompt = {
        let orchestrator_hint = "\n\nTool usage strategy:\n- For simple lookups (search memory, search KB, search web): use the tool directly. Do NOT spawn a sub-agent for a single search.\n- For reading a webpage: use web_read directly.\n- For saving a fact: use memory_save directly.\n- For writing a document: use workspace_write directly.\n- Spawn a sub-agent ONLY when:\n  a) The task requires multiple sequential steps (research \u{2192} analyze \u{2192} write)\n  b) Multiple independent tasks can run in parallel\n  c) The task needs a specialized skill or different model\n- After spawning sub-agents, WAIT before checking status. Sub-agents need at least 30 seconds to complete meaningful work. Spawn all needed sub-agents first, do other work, then check their status.";
        Some(system_prompt.unwrap_or_default() + orchestrator_hint)
    };

    // Inject current date so the agent knows today's date (for web searches, etc.)
    let system_prompt = {
        let date_str = chrono::Local::now().format("%Y-%m-%d %A").to_string();
        let date_line = format!("\n\nCurrent date: {}.", date_str);
        Some(system_prompt.unwrap_or_default() + &date_line)
    };

    // Inject tool routing hint — helps the model choose the right search tool
    let system_prompt = {
        let routing = build_tool_routing_hint(kb_id.is_some(), web_search_enabled, true);
        if routing.is_empty() { system_prompt } else { Some(system_prompt.unwrap_or_default() + &routing) }
    };

    // For resume, we need the API key and base_url. Read from the chat's provider settings.
    // This is a simplified approach — read from the settings store.
    let (api_key, base_url) = match get_provider_credentials(&app, &pool, &chat_id).await {
        Ok(creds) => creds,
        Err(e) => {
            finish_run(&app, &pool, &run_id, &chat_id, "failed", current_iterations, Some(e), None).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }
    };

    // Build query processing config now that api_key is available
    let kb_query_config: Option<crate::services::query_processor::QueryProcessingConfig> =
        kb_query_config_fields.map(|(qr, qd, qv)| {
            crate::commands::knowledge_base::build_query_config_from_kb(
                qr, qd, qv,
                Some(model.clone()),
                Some(api_key.clone()),
                base_url.clone(),
            )
        });

    let base = base_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
    let url = uni_llm::build_llm_url(base.trim_end_matches('/'));
    let headers = uni_llm::build_llm_headers(&api_key);

    let client = match build_http_client(&app, None).await {
        Ok(c) => c,
        Err(e) => {
            finish_run(&app, &pool, &run_id, &chat_id, "failed", current_iterations, Some(e), None).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }
    };

    // Build MCP tools
    let mcp_tools = mcp_mgr.get_all_tools().await;
    let (mut tools_json, tool_name_map) = if !mcp_tools.is_empty() {
        let (openai_tools, name_map) = mcp_tools_to_openai(&mcp_tools);
        (Some(serde_json::Value::Array(openai_tools)), name_map)
    } else {
        (None, std::collections::HashMap::new())
    };

    // Inject built-in memory + workspace tools (+ web search + kb search if enabled)
    {
        let mut builtin_tools = memory_tool_definitions();
        builtin_tools.extend(workspace_tool_definitions());
        if web_search_enabled {
            builtin_tools.extend(web_search_tool_definitions());
        }
        if kb_id.is_some() {
            builtin_tools.extend(kb_search_tool_definitions());
        }
        if let Some(ref mut tools) = tools_json {
            if let Some(arr) = tools.as_array_mut() {
                arr.extend(builtin_tools);
            }
        } else {
            tools_json = Some(serde_json::Value::Array(builtin_tools));
        }
    }

    // Load skills for resume
    let chat_skills_resume = crate::commands::skills::get_chat_skills_impl(&pool, &chat_id)
        .await
        .unwrap_or_default();
    let skill_content = if chat_skills_resume.is_empty() {
        None
    } else {
        Some(
            chat_skills_resume
                .iter()
                .map(|s| format!("<skill name=\"{}\">\n{}\n</skill>", s.name, s.content))
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    };

    let mut iteration = current_iterations;
    let mut accumulated_content = String::new();

    // Load existing trace from DB (from previous steps) or create new
    let mut run_trace: AgentRunTrace = {
        let existing: Option<String> = sqlx::query_scalar("SELECT trace FROM agent_runs WHERE id = ?")
            .bind(&run_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None)
            .flatten();
        match existing.and_then(|s| serde_json::from_str::<AgentRunTrace>(&s).ok()) {
            Some(t) => t,
            None => AgentRunTrace::new(run_id.clone(), chat_id.clone(), model.clone(), chrono::Utc::now().timestamp()),
        }
    };

    // Find the last message's id for parent chaining
    let last_msg_id: String = sqlx::query_scalar(
        "SELECT id FROM messages WHERE chat_id = ? ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(&chat_id)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| String::new());

    let last_parent_id = last_msg_id;

    // Single iteration (step-by-step continues one step at a time)
    if cancel_token.is_cancelled() {
        run_trace.finish("cancelled", Some("cancelled".to_string()));
        finish_run(&app, &pool, &run_id, &chat_id, "cancelled", iteration, None, Some(&run_trace)).await;
        cleanup_cancel_token(&cancel_tokens, &run_id).await;
        return;
    }

    if iteration >= max_iterations {
        let _ = app.emit("agent-run-limit", AgentRunLimitPayload {
            run_id: run_id.clone(),
            iteration,
            max_iterations,
        });
        run_trace.finish("completed", Some("max_steps".to_string()));
        finish_run(&app, &pool, &run_id, &chat_id, "completed", iteration, Some("Max iterations reached".to_string()), Some(&run_trace)).await;
        cleanup_cancel_token(&cancel_tokens, &run_id).await;
        return;
    }

    iteration += 1;
    let step_start = std::time::Instant::now();
    let step_started_at = chrono::Utc::now().timestamp();

    let _ = app.emit("agent-step-start", AgentStepPayload {
        run_id: run_id.clone(),
        chat_id: chat_id.clone(),
        iteration,
    });

    let messages = match build_agent_messages(&pool, &app, &chat_id, &system_prompt, mem_store.as_ref().as_ref(), &skill_content, None, project_id.as_deref(), kb_id.as_deref(), kb_store.as_ref().as_ref(), kb_provider.as_deref(), &client, kb_query_config.as_ref(), kb_reranker_config.as_ref()).await {
        Ok(m) => m,
        Err(e) => {
            run_trace.finish("failed", None);
            finish_run(&app, &pool, &run_id, &chat_id, "failed", iteration, Some(e), Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
            return;
        }
    };

    // Trim messages to fit context window
    let trim_config = context_manager::TrimConfig {
        model_context_limit: context_manager::get_model_context_limit(&model),
        ..Default::default()
    };
    let trim_result = context_manager::trim_messages(&messages, &trim_config);
    if trim_result.was_trimmed {
        eprintln!(
            "[Agent Resume] Context trimmed: {} → {} messages, {} → {} tokens ({:.0}% of {})",
            trim_result.original_count, trim_result.trimmed_count,
            trim_result.original_tokens, trim_result.final_tokens,
            trim_result.usage_ratio * 100.0, trim_config.model_context_limit
        );
    }
    let _ = app.emit("agent-context-usage", serde_json::json!({
        "runId": run_id,
        "usageRatio": trim_result.usage_ratio,
        "totalTokens": trim_result.final_tokens,
        "modelLimit": trim_config.model_context_limit,
        "wasTrimmed": trim_result.was_trimmed,
    }));
    let messages = trim_result.messages;

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "stream_options": {"include_usage": true}
    });

    if let Some(ref tools) = tools_json {
        body["tools"] = tools.clone();
    }

    let config = LlmStreamConfig {
        client: &client,
        url: &url,
        headers: headers.clone(),
        body: &body,
        cancel_token: &cancel_token,
        event_prefix: "agent-stream",
        app: &app,
    };

    let result = stream_llm_call(&config).await;

    match result {
        Err(e) if e == "__cancelled__" => {
            let _ = app.emit("agent-stream-done", StreamDonePayload { full_content: accumulated_content });
            run_trace.finish("cancelled", Some("cancelled".to_string()));
            finish_run(&app, &pool, &run_id, &chat_id, "cancelled", iteration, None, Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
        }
        Err(e) => {
            let _ = app.emit("agent-stream-error", StreamErrorPayload { error: e.clone() });
            run_trace.finish("failed", None);
            finish_run(&app, &pool, &run_id, &chat_id, "failed", iteration, Some(e), Some(&run_trace)).await;
            cleanup_cancel_token(&cancel_tokens, &run_id).await;
        }
        Ok(stream_result) => {
            let llm_duration_ms = step_start.elapsed().as_millis() as u64;
            accumulated_content.push_str(&stream_result.content);

            let asst_msg_id = uni_common::generate_id();
            let msg_now = uni_common::now_unix_secs();

            let pt = stream_result.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
            let ct = stream_result.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
            let provider = provider_from_base_url(&base);
            let cat_id = catalog_id(provider, &model);
            let cost = compute_cost_from_catalog(&pool, &cat_id, pt, ct).await;

            let llm_trace = LlmCallTrace {
                input_tokens: pt as usize,
                output_tokens: ct as usize,
                cost,
                duration_ms: llm_duration_ms,
                had_tool_calls: !stream_result.tool_calls.is_empty(),
                response_preview: safe_truncate_chars(&stream_result.content, 200),
            };
            let mut step_tool_traces: Vec<ToolCallTrace> = Vec::new();

            let _ = sqlx::query(
                "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'assistant', ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)"
            )
            .bind(&asst_msg_id).bind(&chat_id).bind(&stream_result.content)
            .bind(&last_parent_id).bind(msg_now).bind(&model)
            .bind(pt as i64)
            .bind(ct as i64)
            .bind(cost)
            .bind(iteration).bind(&run_id)
            .execute(&pool).await;

            let _ = write_cost_ledger(
                &pool,
                &chat_id,
                Some(&run_id),
                None,
                &model,
                provider,
                pt,
                ct,
                cost,
                "agent",
            ).await;

            let run_cost: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE agent_run_id = ?")
                .bind(&run_id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
                .and_then(|row| row.try_get::<f64, _>("total").ok())
                .unwrap_or(0.0);
            let _ = sqlx::query("UPDATE agent_runs SET cost = ? WHERE id = ?")
                .bind(run_cost).bind(&run_id)
                .execute(&pool).await;

            // Update active_child_map
            {
                let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
                    .bind(&chat_id)
                    .fetch_one(&pool)
                    .await
                    .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
                    .unwrap_or_else(|_| "{}".to_string());
                let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
                map.insert(last_parent_id.clone(), asst_msg_id.clone());
                let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
                let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
                    .bind(&new_map).bind(&chat_id)
                    .execute(&pool).await;
            }

            if stream_result.tool_calls.is_empty() {
                let step_trace = AgentStepTrace {
                    step: iteration as usize,
                    started_at: step_started_at,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    llm_call: llm_trace,
                    tool_calls: step_tool_traces,
                };
                let _ = app.emit("agent-step-trace", &step_trace);
                run_trace.add_step(step_trace);
                run_trace.finish("completed", Some("natural".to_string()));
                // Emit step-done for the final iteration so frontend reloads DB messages
                let _ = app.emit("agent-step-done", AgentStepPayload {
                    run_id: run_id.clone(),
                    chat_id: chat_id.clone(),
                    iteration,
                });
                let _ = app.emit("agent-stream-done", StreamDonePayload { full_content: accumulated_content });
                finish_run(&app, &pool, &run_id, &chat_id, "completed", iteration, None, Some(&run_trace)).await;
                cleanup_cancel_token(&cancel_tokens, &run_id).await;
                // Auto-extract memories in background
                let ext_app = app.clone();
                let ext_pool = pool.clone();
                let ext_mem = mem_store.clone();
                let ext_chat = chat_id.clone();
                let ext_model = model.clone();
                let ext_key = api_key.clone();
                let ext_base = base.clone();
                let ext_pid = project_id.clone();
                tokio::spawn(async move {
                    extract_memories_from_run(&ext_app, &ext_pool, &ext_mem, &ext_chat, &ext_model, &ext_key, &ext_base, ext_pid.as_deref()).await;
                });
                return;
            }

            // Execute tool calls
            let mut tool_parent_id = asst_msg_id;
            for tool_call in &stream_result.tool_calls {
                let (server_id, tool_name) = parse_tool_call_name(&tool_call.name, &tool_name_map);

                let _ = app.emit("agent-tool-call", StreamToolCallPayload {
                    tool_call_id: tool_call.id.clone(),
                    server_id: server_id.clone(),
                    tool_name: tool_name.clone(),
                    arguments: tool_call.arguments.clone(),
                });

                let args: serde_json::Value =
                    serde_json::from_str(&tool_call.arguments).unwrap_or(serde_json::json!({}));

                let tool_start = std::time::Instant::now();

                // Intercept built-in tools (memory + workspace + web search)
                let (result_text, is_error) = if tool_call.name == "memory_save" || tool_call.name == "memory_search" {
                    handle_memory_tool(&app, &pool, mem_store.as_ref().as_ref(), &tool_call.name, &args, Some(&chat_id), project_id.as_deref()).await
                } else if tool_call.name == "workspace_write" || tool_call.name == "workspace_read" || tool_call.name == "workspace_list" {
                    let res = handle_workspace_tool(&pool, &chat_id, project_id.as_deref(), &run_id, &tool_call.name, &args).await;
                    if tool_call.name == "workspace_write" && !res.1 {
                        let _ = app.emit("workspace-changed", serde_json::json!({
                            "chatId": &chat_id,
                            "projectId": project_id.as_deref()
                        }));
                    }
                    res
                } else if tool_call.name == "web_search" || tool_call.name == "web_read" {
                    handle_web_search_tool(&app, &tool_call.name, &args, &web_search_provider, ws_tavily_key.as_deref(), ws_brave_key.as_deref()).await
                } else if tool_call.name == "kb_search" {
                    if let Some(ref kb_id_val) = kb_id {
                        handle_kb_search_tool(&pool, kb_store.as_ref().as_ref(), kb_id_val, &args, kb_provider.as_deref(), &client, kb_query_config.as_ref(), kb_reranker_config.as_ref()).await
                    } else {
                        ("Knowledge base not attached to this chat.".to_string(), true)
                    }
                } else {
                    let result = mcp_mgr.call_tool(&server_id, &tool_name, args).await;
                    match result {
                        Ok(r) => {
                            let text = r.content.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");
                            (text, r.is_error)
                        }
                        Err(e) => (e.to_string(), true),
                    }
                };

                let tool_duration_ms = tool_start.elapsed().as_millis() as u64;
                step_tool_traces.push(ToolCallTrace {
                    tool_name: tool_call.name.clone(),
                    arguments_preview: safe_truncate_chars(&tool_call.arguments, 200),
                    result_preview: safe_truncate_chars(&result_text, 200),
                    duration_ms: tool_duration_ms,
                    is_error,
                    is_builtin: is_builtin_tool(&tool_call.name),
                });

                let _ = app.emit("agent-tool-result", StreamToolResultPayload {
                    tool_call_id: tool_call.id.clone(),
                    result: result_text.clone(),
                    is_error,
                });

                let tool_msg_id = uni_common::generate_id();
                let tool_content = serde_json::json!({
                    "tool_call_id": tool_call.id,
                    "tool_name": tool_name,
                    "server_id": server_id,
                    "arguments": tool_call.arguments,
                    "result": result_text,
                    "is_error": is_error
                }).to_string();
                let tool_now = uni_common::now_unix_secs();

                let _ = sqlx::query(
                    "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'tool', ?, ?, ?, '', 0, 0, 0.0, 0, ?, ?)"
                )
                .bind(&tool_msg_id).bind(&chat_id).bind(&tool_content)
                .bind(&tool_parent_id).bind(tool_now)
                .bind(iteration).bind(&run_id)
                .execute(&pool).await;

                // Update active_child_map
                {
                    let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
                        .bind(&chat_id)
                        .fetch_one(&pool)
                        .await
                        .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
                        .unwrap_or_else(|_| "{}".to_string());
                    let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
                    map.insert(tool_parent_id.clone(), tool_msg_id.clone());
                    let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
                    let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
                        .bind(&new_map).bind(&chat_id)
                        .execute(&pool).await;
                }

                tool_parent_id = tool_msg_id;
            }

            // Build and emit step trace
            let step_trace = AgentStepTrace {
                step: iteration as usize,
                started_at: step_started_at,
                duration_ms: step_start.elapsed().as_millis() as u64,
                llm_call: llm_trace,
                tool_calls: step_tool_traces,
            };
            let _ = app.emit("agent-step-trace", &step_trace);
            run_trace.add_step(step_trace);

            // Pause again (step-by-step continues one step per resume)
            let _ = app.emit("agent-step-pause", AgentStepPayload {
                run_id: run_id.clone(),
                chat_id: chat_id.clone(),
                iteration,
            });
            // Save trace so far for resume to pick up
            let trace_json = serde_json::to_string(&run_trace).unwrap_or_default();
            let _ = sqlx::query("UPDATE agent_runs SET status = 'paused', iterations = ?, trace = ? WHERE id = ?")
                .bind(iteration).bind(&trace_json).bind(&run_id)
                .execute(&pool).await;
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Build messages array from DB for the agent API call.
/// Returns Vec<serde_json::Value> with proper OpenAI tool_calls / tool format.
async fn build_agent_messages(
    pool: &Pool,
    app: &AppHandle,
    chat_id: &str,
    system_prompt: &Option<String>,
    memory_store: Option<&MemoryVectorStore>,
    skill_content: &Option<String>,
    scope_agent_run_id: Option<&str>,
    project_id: Option<&str>,
    kb_id: Option<&str>,
    kb_vector_store: Option<&KbVectorStore>,
    kb_provider: Option<&dyn EmbeddingProvider>,
    http_client: &reqwest::Client,
    kb_query_config: Option<&crate::services::query_processor::QueryProcessingConfig>,
    kb_reranker_config: Option<&crate::services::kb_search_orchestrator::RerankerConfig>,
) -> Result<Vec<serde_json::Value>, String> {
    // When scoped to a specific agent_run, only load pre-plan messages + this run's messages
    // (used for parallel plan task execution to isolate message contexts)
    let mut raw: Vec<(String, String)> = Vec::new();

    if let Some(run_id) = scope_agent_run_id {
        let rows = sqlx::query(
            "SELECT role, content FROM messages WHERE chat_id = ? AND (agent_run_id IS NULL OR agent_run_id = ?) ORDER BY timestamp ASC"
        )
        .bind(chat_id)
        .bind(run_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        for row in &rows {
            let role: String = row.get("role");
            let content: String = row.get("content");
            raw.push((role, content));
        }
    } else {
        // Full active-branch tree traversal (original behavior)
        let rows = sqlx::query(
            "SELECT id, role, content, parent_id FROM messages WHERE chat_id = ? ORDER BY timestamp ASC"
        )
        .bind(chat_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        // Load active_child_map from chat
        let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
            .bind(chat_id)
            .fetch_one(pool)
            .await
            .map(|row| row.try_get::<String, _>("active_child_map").unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|_| "{}".to_string());
        let active_child_map: std::collections::HashMap<String, String> =
            serde_json::from_str(&map_str).unwrap_or_default();

        // Build children map: parent_id → Vec<(id, role, content)>
        let mut children_map: std::collections::HashMap<Option<String>, Vec<(String, String, String)>> =
            std::collections::HashMap::new();
        for row in &rows {
            let id: String = row.get("id");
            let role: String = row.get("role");
            let content: String = row.get("content");
            let parent_id: Option<String> = row.try_get::<Option<String>, _>("parent_id")
                .unwrap_or(None);
            children_map.entry(parent_id).or_default().push((id, role, content));
        }

        // Walk active branch path (same algorithm as build_active_path in database.rs)
        let roots = children_map.get(&None)
            .or_else(|| children_map.get(&Some(String::new())))
            .cloned()
            .unwrap_or_default();

        if !roots.is_empty() {
            let active_root = if roots.len() == 1 {
                &roots[0]
            } else if let Some(active_id) = active_child_map.get("") {
                roots.iter().find(|(id, _, _)| id == active_id).unwrap_or(&roots[0])
            } else {
                &roots[0]
            };

            raw.push((active_root.1.clone(), active_root.2.clone()));
            let mut current_id = active_root.0.clone();

            loop {
                let children = match children_map.get(&Some(current_id.clone())) {
                    Some(c) if !c.is_empty() => c,
                    _ => break,
                };

                let active_child = if children.len() == 1 {
                    &children[0]
                } else if let Some(active_id) = active_child_map.get(&current_id) {
                    children.iter().find(|(id, _, _)| id == active_id)
                        .unwrap_or(children.last().unwrap())
                } else {
                    children.last().unwrap()
                };

                raw.push((active_child.1.clone(), active_child.2.clone()));
                current_id = active_child.0.clone();
            }
        }
    }

    let mut messages: Vec<serde_json::Value> = Vec::new();

    // System prompt first
    if let Some(ref sp) = system_prompt {
        if !sp.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": sp}));
        }
    }

    // Inject skill instructions
    if let Some(ref sc) = skill_content {
        if !sc.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": sc}));
        }
    }

    // Auto-inject relevant memories based on last user message + active skill context
    if let Some(ms) = memory_store {
        let last_user_text = raw.iter().rev().find(|(r, _)| r == "user").map(|(_, c)| c.as_str()).unwrap_or("");
        // Enrich search query with skill names for better relevance
        let search_query = if let Some(ref sc) = skill_content {
            let skill_names: Vec<&str> = sc.match_indices("<skill name=\"")
                .filter_map(|(pos, _)| {
                    let start = pos + 13;
                    sc[start..].find('"').map(|end| &sc[start..start + end])
                })
                .collect();
            if skill_names.is_empty() {
                last_user_text.to_string()
            } else {
                format!("{} [context: {}]", last_user_text, skill_names.join(", "))
            }
        } else {
            last_user_text.to_string()
        };
        if !search_query.is_empty() {
            let emb = crate::services::embedding_helper::get_embedding_provider(app).await;
            let pref = emb.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
            if let Ok(results) = crate::commands::agent_memory::search_memories_impl(
                pool,
                Some(ms),
                &search_query,
                10,
                project_id,
                pref,
            )
            .await
            {
                if !results.is_empty() {
                    let mut memory_block = String::from("<agent_memory>\nRelevant facts from your memory:\n");
                    for r in &results {
                        memory_block.push_str(&format!("- [{}] {}\n", r.memory.category, r.memory.content));
                    }
                    memory_block.push_str("Use memory_save tool to store new important facts.\n</agent_memory>");
                    messages.push(serde_json::json!({"role": "system", "content": memory_block}));
                }
            }
        }
    }

    // Inject KB context if chat has an attached knowledge base
    if let (Some(kb_id_val), Some(store)) = (kb_id, kb_vector_store) {
        let last_user_text = raw.iter().rev().find(|(r, _)| r == "user").map(|(_, c)| c.as_str()).unwrap_or("");
        if !last_user_text.is_empty() {
            let kb_row = sqlx::query("SELECT name, retrieval_top_k, retrieval_min_score, \
                 context_token_budget, context_sentence_extraction, context_redundancy_removal, embedding_dimensions \
                 FROM knowledge_bases WHERE id = ?")
                .bind(kb_id_val)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
            if let Some(row) = kb_row {
                let kb_name: String = row.get("name");
                let top_k: i64 = row.get("retrieval_top_k");
                let min_score: f64 = row.get("retrieval_min_score");
                let emb_dims: Option<i32> = row
                    .try_get::<i64, _>("embedding_dimensions")
                    .ok()
                    .map(|v| v as i32);
                let config = KbSearchConfig {
                    top_k: top_k as usize,
                    min_score: min_score as f32,
                    ..Default::default()
                };
                let default_qc = crate::services::query_processor::QueryProcessingConfig::default();
                let qc = kb_query_config.unwrap_or(&default_qc);
                let default_rc = crate::services::kb_search_orchestrator::RerankerConfig::default();
                let rc = kb_reranker_config.unwrap_or(&default_rc);
                if let Ok(orchestrated) = crate::services::kb_search_orchestrator::search_kb_orchestrated(
                    pool,
                    store,
                    kb_id_val,
                    last_user_text,
                    &config,
                    qc,
                    rc,
                    http_client,
                    kb_provider,
                    emb_dims,
                )
                .await
                {
                    if !orchestrated.results.is_empty() {
                        let mut query_trace = orchestrated.query_trace;
                        let ctx_budget: i64 = row.try_get("context_token_budget").unwrap_or(4000);
                        let ctx_sentence: bool = row.try_get::<bool, _>("context_sentence_extraction").unwrap_or(true);
                        let ctx_redundancy: bool = row.try_get::<bool, _>("context_redundancy_removal").unwrap_or(true);
                        let opt_config = crate::commands::knowledge_base::build_optimization_config_from_kb(
                            ctx_budget, ctx_sentence, ctx_redundancy,
                        );
                        let (optimized, opt_trace) = crate::services::context_optimizer::optimize_context(
                            last_user_text, &orchestrated.results, &opt_config,
                        );
                        query_trace.optimization = Some(opt_trace);
                        let kb_context = rag_context::build_rag_context_optimized(&optimized, &kb_name);
                        messages.push(serde_json::json!({"role": "system", "content": kb_context}));

                        // Note: trace is captured in query_trace but not stored on agent messages
                        // since assistant_msg_id is not available in build_agent_messages context
                    }
                }
            }
        }
    }

    let mut i = 0;
    while i < raw.len() {
        let (ref role, ref content) = raw[i];

        if role == "tool" {
            // Tool message: format as proper role:"tool" with tool_call_id
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                let tool_call_id = parsed.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                let result_text = parsed.get("result").and_then(|v| v.as_str()).unwrap_or(content);
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": result_text
                }));
            } else {
                // Fallback: can't parse, send as user context
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": format!("[Tool result]: {}", content)
                }));
            }
            i += 1;
        } else if role == "assistant" {
            // Peek ahead: if next messages are "tool", reconstruct tool_calls array
            let mut tool_calls_json = Vec::new();
            let mut j = i + 1;
            while j < raw.len() && raw[j].0 == "tool" {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw[j].1) {
                    let tc_id = parsed.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                    let tc_name = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                    let tc_args = parsed.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                    tool_calls_json.push(serde_json::json!({
                        "id": tc_id,
                        "type": "function",
                        "function": {
                            "name": tc_name,
                            "arguments": tc_args
                        }
                    }));
                }
                j += 1;
            }

            if tool_calls_json.is_empty() {
                // Normal assistant message (no tool calls follow)
                messages.push(serde_json::json!({"role": "assistant", "content": content}));
            } else {
                // Assistant message with tool_calls
                let mut msg = serde_json::json!({
                    "role": "assistant",
                    "tool_calls": tool_calls_json
                });
                if !content.is_empty() {
                    msg["content"] = serde_json::json!(content);
                }
                messages.push(msg);
            }
            i += 1;
        } else {
            // user, system, etc.
            messages.push(serde_json::json!({"role": role, "content": content}));
            i += 1;
        }
    }

    // Log total context size for monitoring
    let total_system_chars: usize = messages.iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .map(|c| c.len())
        .sum();

    if total_system_chars > 0 {
        let skill_chars = skill_content.as_ref().map(|s| s.len()).unwrap_or(0);
        let sp_chars = system_prompt.as_ref().map(|s| s.len()).unwrap_or(0);
        log::info!(
            "[build_agent_messages] chat={} system_context_chars={} (skills={}, memory={}, system_prompt={})",
            chat_id,
            total_system_chars,
            skill_chars,
            total_system_chars.saturating_sub(skill_chars).saturating_sub(sp_chars),
            sp_chars,
        );
    }

    const MAX_SYSTEM_CONTEXT_CHARS: usize = 12000;
    if total_system_chars > MAX_SYSTEM_CONTEXT_CHARS {
        log::warn!(
            "[build_agent_messages] system context too large ({} chars > {} limit), consider reducing skills or memory",
            total_system_chars,
            MAX_SYSTEM_CONTEXT_CHARS
        );
    }

    Ok(messages)
}

/// Get provider credentials (api_key, base_url) for a chat
pub(crate) async fn get_provider_credentials(
    app: &AppHandle,
    pool: &Pool,
    chat_id: &str,
) -> Result<(String, Option<String>), String> {
    use tauri_plugin_store::StoreExt;
    let provider_id: String = sqlx::query("SELECT provider_id FROM chats WHERE id = ?")
        .bind(chat_id)
        .fetch_one(pool)
        .await
        .map(|row| row.try_get("provider_id").unwrap_or_else(|_| "openrouter".to_string()))
        .unwrap_or_else(|_| "openrouter".to_string());

    if provider_id == "openrouter" {
        let store = app.store("settings.json").map_err(|e: tauri_plugin_store::Error| e.to_string())?;
        let api_key = store.get("apiKey")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from))
            .unwrap_or_default();
        Ok((api_key, None))
    } else if provider_id == "ollama" {
        let store = app.store("settings.json").map_err(|e: tauri_plugin_store::Error| e.to_string())?;
        let url = store.get("ollamaUrl")
            .and_then(|v: serde_json::Value| v.as_str().map(String::from))
            .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
        Ok((String::new(), Some(url)))
    } else {
        // Custom provider
        let row = sqlx::query("SELECT base_url, api_key FROM custom_providers WHERE id = ?")
            .bind(&provider_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
        match row {
            Some(r) => {
                let base_url: String = r.get("base_url");
                let api_key: String = r.get("api_key");
                Ok((api_key, Some(base_url)))
            }
            None => Ok((String::new(), None)),
        }
    }
}

async fn finish_run(
    app: &AppHandle,
    pool: &Pool,
    run_id: &str,
    chat_id: &str,
    status: &str,
    iterations: i64,
    error: Option<String>,
    trace: Option<&AgentRunTrace>,
) {
    let now = uni_common::now_unix_secs();

    let run_cost: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE agent_run_id = ?")
        .bind(run_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|row| row.try_get::<f64, _>("total").ok())
        .unwrap_or(0.0);

    let trace_json: Option<String> = trace.and_then(|t| serde_json::to_string(t).ok());

    let _ = sqlx::query(
        "UPDATE agent_runs SET status = ?, iterations = ?, finished_at = ?, error = ?, cost = ?, trace = ? WHERE id = ?"
    )
    .bind(status).bind(iterations).bind(now).bind(&error).bind(run_cost).bind(&trace_json).bind(run_id)
    .execute(pool).await;

    // Update chat's last_run_status for sidebar indicators
    let _ = sqlx::query("UPDATE chats SET last_run_status = ? WHERE id = ?")
        .bind(status).bind(chat_id)
        .execute(pool).await;

    // Emit trace event if available
    if let Some(t) = trace {
        let _ = app.emit("agent-run-trace", t);
    }

    let _ = app.emit("agent-run-finished", AgentRunFinishedPayload {
        run_id: run_id.to_string(),
        status: status.to_string(),
        iterations,
        error,
    });

    // Notify Telegram handler if waiting
    if let Some(notifier) = app.try_state::<crate::TelegramRunNotifier>() {
        if let Some(tx) = notifier.write().await.remove(run_id) {
            let _ = tx.send(status.to_string());
        }
    }
}

async fn cleanup_cancel_token(cancel_tokens: &AgentCancelTokens, run_id: &str) {
    let mut tokens = cancel_tokens.write().await;
    tokens.remove(run_id);
}

// ─── Built-in Memory Tools ──────────────────────────────────────

fn memory_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "memory_save",
                "description": "Save an important fact to your personal memory for future conversations. Use for: user preferences, project decisions, key information the user shared, things you should remember. Do NOT use for long documents or code — use workspace_write for those.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The fact or information to remember"
                        },
                        "category": {
                            "type": "string",
                            "enum": ["fact", "decision", "preference", "context", "learning"],
                            "description": "Category: fact (objective info), decision (choices made), preference (user likes/dislikes), context (project/situation), learning (lessons learned)"
                        }
                    },
                    "required": ["content", "category"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "memory_search",
                "description": "Search your personal memory for facts you previously saved about the user, their preferences, project details, or past conversations. Use this for questions like 'what is the user\\'s favorite language?' or 'what did we discuss about X?' This searches YOUR saved notes — not documents, not the internet.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "What to search for in memory"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Max results to return (default 5)"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
    ]
}

fn workspace_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "workspace_write",
                "description": "Save a document, code file, report, or other artifact to the shared workspace. Use for structured output that the user will reference later: generated code, reports, analysis results, plans. Do NOT use for saving short facts — use memory_save for those.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Artifact name/filename (e.g. 'auth.rs', 'report.md')" },
                        "content": { "type": "string", "description": "Full content of the artifact" },
                        "content_type": { "type": "string", "enum": ["code", "text", "data"], "description": "Type of content" }
                    },
                    "required": ["name", "content", "content_type"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "workspace_read",
                "description": "Read a previously saved artifact from the workspace by name.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Artifact name to read" }
                    },
                    "required": ["name"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "workspace_list",
                "description": "List all artifacts currently in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
    ]
}

async fn handle_workspace_tool(
    pool: &Pool,
    chat_id: &str,
    project_id: Option<&str>,
    run_id: &str,
    tool_name: &str,
    args: &serde_json::Value,
) -> (String, bool) {
    use crate::commands::workspace::{
        create_artifact_impl, list_artifacts_impl, read_artifact_impl, update_artifact_impl,
    };

    match tool_name {
        "workspace_write" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let content_type = args.get("content_type").and_then(|v| v.as_str()).unwrap_or("text");
            if name.is_empty() {
                return ("Error: name is required".to_string(), true);
            }
            match read_artifact_impl(pool, chat_id, project_id, name).await {
                Ok(Some(_)) => {
                    match update_artifact_impl(pool, chat_id, project_id, name, content, Some(run_id)).await {
                        Ok(()) => (format!("Updated artifact '{}' in workspace", name), false),
                        Err(e) => (format!("Failed to update artifact: {}", e), true),
                    }
                }
                Ok(None) => {
                    match create_artifact_impl(pool, chat_id, project_id, name, content_type, Some(content), Some(run_id)).await {
                        Ok(_) => (format!("Created artifact '{}' in workspace", name), false),
                        Err(e) => (format!("Failed to create artifact: {}", e), true),
                    }
                }
                Err(e) => (format!("Failed to check artifact: {}", e), true),
            }
        }
        "workspace_read" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                return ("Error: name is required".to_string(), true);
            }
            match read_artifact_impl(pool, chat_id, project_id, name).await {
                Ok(Some(artifact)) => {
                    (artifact.content.unwrap_or_else(|| "(empty)".to_string()), false)
                }
                Ok(None) => (format!("Artifact '{}' not found in workspace", name), false),
                Err(e) => (format!("Failed to read artifact: {}", e), true),
            }
        }
        "workspace_list" => {
            match list_artifacts_impl(pool, chat_id, project_id).await {
                Ok(artifacts) => {
                    if artifacts.is_empty() {
                        ("Workspace is empty".to_string(), false)
                    } else {
                        let list: Vec<String> = artifacts
                            .iter()
                            .map(|a| {
                                format!(
                                    "- {} [{}] ({} bytes)",
                                    a.name,
                                    a.content_type,
                                    a.content.as_ref().map(|c| c.len()).unwrap_or(0)
                                )
                            })
                            .collect();
                        (format!("Workspace artifacts:\n{}", list.join("\n")), false)
                    }
                }
                Err(e) => (format!("Failed to list artifacts: {}", e), true),
            }
        }
        _ => ("Unknown workspace tool".to_string(), true),
    }
}

// ─── Built-in Web Search Tools ──────────────────────────────────

fn web_search_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the internet for current information. Use for: recent events, real-time data, topics not covered in the Knowledge Base or memory, general knowledge questions, fact-checking. This searches the web — not the user's documents, not your memory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query. Be specific and concise."
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_read",
                "description": "Read the full content of a specific web page URL. Use after web_search to get detailed content from a search result, or when the user provides a specific URL to read.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to read"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
    ]
}

async fn handle_web_search_tool(
    app: &AppHandle,
    tool_name: &str,
    args: &serde_json::Value,
    provider: &str,
    tavily_api_key: Option<&str>,
    brave_api_key: Option<&str>,
) -> (String, bool) {
    match tool_name {
        "web_search" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            if query.is_empty() {
                return ("Error: query is required".to_string(), true);
            }
            let client = match build_http_client(app, Some(std::time::Duration::from_secs(15))).await {
                Ok(c) => c,
                Err(e) => return (format!("Search failed: {}", e), true),
            };
            let results = match provider {
                "tavily" => {
                    let key = tavily_api_key.unwrap_or("");
                    if key.is_empty() {
                        return ("Search failed: Tavily API key not configured".to_string(), true);
                    }
                    web_search::search_tavily(&client, key, query, 5).await
                }
                "brave" => {
                    let key = brave_api_key.unwrap_or("");
                    if key.is_empty() {
                        return ("Search failed: Brave API key not configured".to_string(), true);
                    }
                    web_search::search_brave(&client, key, query, 5).await
                }
                _ => web_search::search_uni(query, 5, &client).await,
            };
            match results {
                Ok(items) => {
                    if items.is_empty() {
                        return ("No web results found. Consider: try kb_search if the topic might be in the user's documents, or rephrase your search query.".to_string(), false);
                    }
                    let mut output = format!("Search results for \"{}\":\n\n", query);
                    for (i, item) in items.iter().take(5).enumerate() {
                        output.push_str(&format!(
                            "{}. [{}]({})\n   {}\n\n",
                            i + 1,
                            item.title,
                            item.url,
                            item.snippet
                        ));
                    }
                    (output.trim_end().to_string(), false)
                }
                Err(e) => (format!("Search failed: {}", e), true),
            }
        }
        "web_read" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url.is_empty() {
                return ("Error: url is required".to_string(), true);
            }
            let client = match build_http_client(app, Some(std::time::Duration::from_secs(15))).await {
                Ok(c) => c,
                Err(e) => return (format!("Failed to read page: {}", e), true),
            };
            match web_content::fetch_content(&client, url, 2000).await {
                Ok(content) => {
                    const MAX_CHARS: usize = 8000;
                    if content.len() > MAX_CHARS {
                        // UTF-8 safe truncation
                        let mut end = MAX_CHARS;
                        while !content.is_char_boundary(end) && end > 0 {
                            end -= 1;
                        }
                        (format!("{}... [truncated]", &content[..end]), false)
                    } else {
                        (content, false)
                    }
                }
                Err(e) => (format!("Failed to read page: {}", e), true),
            }
        }
        _ => ("Unknown web search tool".to_string(), true),
    }
}

fn kb_search_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "kb_search",
                "description": "Search the attached Knowledge Base documents for information. Use this when the user asks about topics that are likely covered in their uploaded documents (documentation, guides, reports, codebases). This searches document content — not your memory, not the internet.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query — what information are you looking for"
                        },
                        "top_k": {
                            "type": "integer",
                            "description": "Number of results to return (default: 5, max: 20)"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
    ]
}

fn build_tool_routing_hint(has_kb: bool, has_web_search: bool, has_memory: bool) -> String {
    let mut hints = vec![];
    if has_memory {
        hints.push("- Questions about user preferences or past decisions \u{2192} memory_search");
        hints.push("- Save a fact for future reference \u{2192} memory_save");
        hints.push("- Produce a document, code, or report \u{2192} workspace_write");
    }
    if has_kb {
        hints.push("- Questions about uploaded documents or project files \u{2192} kb_search");
    }
    if has_web_search {
        hints.push("- Current events, general knowledge, external APIs \u{2192} web_search");
    }
    if hints.is_empty() {
        return String::new();
    }
    let mut result = String::from("\n\n<tool_routing>\nChoose the right tool for each information need:\n");
    for hint in &hints {
        result.push_str(hint);
        result.push('\n');
    }
    if has_kb && has_web_search {
        result.push_str("\nWhen in doubt: try kb_search first for domain questions, web_search for general knowledge.\n");
    }
    if has_memory {
        result.push_str("Use memory_search only for personal/project context you previously saved.\n");
    }
    result.push_str("</tool_routing>");
    result
}

async fn handle_kb_search_tool(
    pool: &Pool,
    kb_vector_store: Option<&KbVectorStore>,
    kb_id: &str,
    args: &serde_json::Value,
    kb_provider: Option<&dyn EmbeddingProvider>,
    http_client: &reqwest::Client,
    query_config: Option<&crate::services::query_processor::QueryProcessingConfig>,
    reranker_config: Option<&crate::services::kb_search_orchestrator::RerankerConfig>,
) -> (String, bool) {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.is_empty() => q,
        _ => return ("Error: query is required".to_string(), true),
    };
    let top_k = args.get("top_k").and_then(|v| v.as_i64()).unwrap_or(5) as usize;
    let top_k = top_k.min(20);

    let store = match kb_vector_store {
        Some(s) => s,
        None => return ("Knowledge base search is not available (vector store not initialized).".to_string(), true),
    };

    let min_score: f64 = sqlx::query_scalar("SELECT retrieval_min_score FROM knowledge_bases WHERE id = ?")
        .bind(kb_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0.3);

    let config = KbSearchConfig {
        top_k,
        min_score: min_score as f32,
        ..Default::default()
    };

    let emb_dims: Option<i32> = sqlx::query_scalar::<_, i64>("SELECT embedding_dimensions FROM knowledge_bases WHERE id = ?")
        .bind(kb_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|v| v as i32);

    let default_qc = crate::services::query_processor::QueryProcessingConfig::default();
    let qc = query_config.unwrap_or(&default_qc);
    let default_rc = crate::services::kb_search_orchestrator::RerankerConfig::default();
    let rc = reranker_config.unwrap_or(&default_rc);

    match crate::services::kb_search_orchestrator::search_kb_orchestrated(
        pool,
        store,
        kb_id,
        query,
        &config,
        qc,
        rc,
        http_client,
        kb_provider,
        emb_dims,
    )
    .await
    {
        Ok(orchestrated) => {
            let results = orchestrated.results;
            if results.is_empty() {
                ("No relevant information found in the knowledge base. Consider: the KB may not cover this topic — try web_search for general information, or rephrase your query with different keywords.".to_string(), false)
            } else {
                let mut output = format!("Found {} relevant results:\n\n", results.len());
                for (i, r) in results.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. [{}] (relevance: {:.2})\n{}\n\n",
                        i + 1,
                        r.document_name,
                        r.score,
                        r.content
                    ));
                }
                (output.trim_end().to_string(), false)
            }
        }
        Err(e) => (format!("Knowledge base search failed: {}", e), true),
    }
}

fn orchestrator_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "spawn_agent",
                "description": "Spawn a sub-agent to work on a specific task. The sub-agent runs autonomously with its own model, skills, and tools. Returns an agent_run_id that you can use to check status and get results.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "goal": { "type": "string", "description": "Clear task description for the sub-agent" },
                        "model": { "type": "string", "description": "Optional: specific model ID. If omitted, model is chosen via routing rules." },
                        "skills": { "type": "array", "items": { "type": "string" }, "description": "Optional: skill names to activate" },
                        "tools": { "type": "array", "items": { "type": "string" }, "description": "Optional: MCP server names to enable" },
                        "context": { "type": "string", "description": "Optional: additional context or instructions" },
                        "workspace_refs": { "type": "array", "items": { "type": "string" }, "description": "Optional: workspace artifact names to include in sub-agent's initial context" },
                        "max_iterations": { "type": "integer", "description": "Optional: max iterations (default 15)" }
                    },
                    "required": ["goal"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "check_agent",
                "description": "Check the status of a spawned sub-agent.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "agent_run_id": { "type": "string", "description": "The run ID returned by spawn_agent" }
                    },
                    "required": ["agent_run_id"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "get_agent_result",
                "description": "Get the final result of a completed sub-agent. Returns the last assistant message, list of workspace artifacts created, cost, and iteration count.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "agent_run_id": { "type": "string", "description": "The run ID returned by spawn_agent" }
                    },
                    "required": ["agent_run_id"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "cancel_agent",
                "description": "Cancel a running sub-agent.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "agent_run_id": { "type": "string", "description": "The run ID to cancel" }
                    },
                    "required": ["agent_run_id"]
                }
            }
        }),
    ]
}

#[allow(clippy::too_many_arguments)]
async fn handle_orchestrator_tool(
    app: AppHandle,
    pool: Pool,
    mcp_mgr: Arc<McpManager>,
    mem_store: Arc<Option<MemoryVectorStore>>,
    kb_store: Arc<Option<KbVectorStore>>,
    cancel_tokens: AgentCancelTokens,
    chat_id: String,
    run_id: String,
    api_key: String,
    base_url: String,
    parent_model: String,
    depth: i64,
    tool_name: &str,
    args: &serde_json::Value,
) -> (String, bool) {
    match tool_name {
        "spawn_agent" => {
            if depth >= 1 {
                return ("Error: sub-agents cannot spawn other sub-agents (max depth = 1)".to_string(), true);
            }
            let config: crate::services::sub_agent::SpawnAgentConfig = match serde_json::from_value(args.clone()) {
                Ok(c) => c,
                Err(e) => return (format!("Invalid spawn_agent args: {}", e), true),
            };
            // spawn_sub_agent creates its own tokio::spawn internally, so we just
            // need to set up the run record and return the ID synchronously-ish.
            match spawn_sub_agent_sync(
                app.clone(), pool.clone(), mcp_mgr.clone(), mem_store.clone(), kb_store.clone(), cancel_tokens.clone(),
                config, run_id.clone(), chat_id.clone(),
                api_key.clone(), base_url.clone(), parent_model.clone(),
            ).await {
                Ok(sub_run_id) => (format!("Sub-agent spawned. Run ID: {}\n\nIMPORTANT: Do NOT call check_agent or get_agent_result for at least 30 seconds. Proceed with other tasks or spawn additional sub-agents first.", sub_run_id), false),
                Err(e) => (format!("Failed to spawn sub-agent: {}", e), true),
            }
        }
        "check_agent" => {
            let target = args.get("agent_run_id").and_then(|v| v.as_str()).unwrap_or("");
            // Poll for up to 10 seconds before returning "still running"
            let mut final_result: Option<(String, bool)> = None;
            for attempt in 0..10 {
                let row = sqlx::query(
                    "SELECT id, status, iterations, max_iterations, error, cost FROM agent_runs WHERE id = ? AND parent_run_id = ?"
                ).bind(target).bind(&run_id).fetch_optional(&pool).await;
                match row {
                    Ok(Some(r)) => {
                        let status: String = r.get("status");
                        let iterations: i64 = r.get("iterations");
                        let max_iterations: i64 = r.get("max_iterations");
                        let error: Option<String> = r.try_get::<Option<String>, _>("error").ok().flatten();
                        let cost: Option<f64> = r.try_get("cost").ok();
                        let info = serde_json::json!({
                            "agent_run_id": target,
                            "status": status,
                            "iterations": iterations,
                            "max_iterations": max_iterations,
                            "error": error,
                            "cost": cost,
                        });
                        if status != "running" {
                            final_result = Some((serde_json::to_string_pretty(&info).unwrap(), false));
                            break;
                        }
                        if attempt == 9 {
                            final_result = Some((format!("{}\nSub-agent is still running. You can check again later or do other work.", serde_json::to_string_pretty(&info).unwrap()), false));
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    Ok(None) => {
                        final_result = Some((format!("Sub-agent '{}' not found or not owned by this orchestrator", target), true));
                        break;
                    }
                    Err(e) => {
                        final_result = Some((format!("DB error: {}", e), true));
                        break;
                    }
                }
            }
            final_result.unwrap_or_else(|| ("Unexpected check_agent error".to_string(), true))
        }
        "get_agent_result" => {
            let target = args.get("agent_run_id").and_then(|v| v.as_str()).unwrap_or("");
            let row = sqlx::query(
                "SELECT status, iterations, cost FROM agent_runs WHERE id = ? AND parent_run_id = ?"
            ).bind(target).bind(&run_id).fetch_optional(&pool).await;
            match row {
                Ok(Some(r)) => {
                    let status: String = r.get("status");
                    if status != "completed" && status != "failed" {
                        return (format!("Sub-agent is still '{}'. Wait for completion.", status), false);
                    }
                    let iterations: i64 = r.get("iterations");
                    let cost: Option<f64> = r.try_get("cost").ok();
                    let last_msg: Option<String> = sqlx::query_scalar(
                        "SELECT content FROM messages WHERE chat_id = ? AND agent_run_id = ? AND role = 'assistant' ORDER BY timestamp DESC LIMIT 1"
                    ).bind(&chat_id).bind(target).fetch_optional(&pool).await.unwrap_or(None);
                    let artifacts: Vec<String> = sqlx::query_scalar(
                        "SELECT name FROM workspace_artifacts WHERE chat_id = ? AND (created_by = ? OR updated_by = ?)"
                    ).bind(&chat_id).bind(target).bind(target).fetch_all(&pool).await.unwrap_or_default();
                    let result = serde_json::json!({
                        "agent_run_id": target,
                        "status": status,
                        "result": last_msg,
                        "artifacts_created": artifacts,
                        "cost": cost,
                        "iterations": iterations,
                    });
                    (serde_json::to_string_pretty(&result).unwrap(), false)
                }
                Ok(None) => (format!("Sub-agent '{}' not found or not owned by this orchestrator", target), true),
                Err(e) => (format!("DB error: {}", e), true),
            }
        }
        "cancel_agent" => {
            let target = args.get("agent_run_id").and_then(|v| v.as_str()).unwrap_or("");
            let exists = sqlx::query("SELECT 1 as x FROM agent_runs WHERE id = ? AND parent_run_id = ?")
                .bind(target).bind(&run_id).fetch_optional(&pool).await;
            match exists {
                Ok(Some(_)) => {
                    let tokens = cancel_tokens.read().await;
                    if let Some(token) = tokens.get(target) {
                        token.cancel();
                        (format!("Sub-agent '{}' cancelled", target), false)
                    } else {
                        (format!("Sub-agent '{}' is not running (no cancel token)", target), false)
                    }
                }
                Ok(None) => (format!("Sub-agent '{}' not found or not owned by this orchestrator", target), true),
                Err(e) => (format!("DB error: {}", e), true),
            }
        }
        _ => ("Unknown orchestrator tool".to_string(), true),
    }
}

/// Spawn a sub-agent with its own agent loop.
#[allow(clippy::too_many_arguments)]
fn spawn_sub_agent_sync(
    app: AppHandle,
    pool: Pool,
    mcp_mgr: Arc<McpManager>,
    mem_store: Arc<Option<MemoryVectorStore>>,
    kb_store: Arc<Option<KbVectorStore>>,
    cancel_tokens: AgentCancelTokens,
    config: crate::services::sub_agent::SpawnAgentConfig,
    parent_run_id: String,
    orchestrator_chat_id: String,
    parent_api_key: String,
    parent_base_url: String,
    parent_model: String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
  Box::pin(async move {
    let sub_run_id = uni_common::generate_id();
    let max_iter = config.max_iterations.unwrap_or(15);
    let now = uni_common::now_unix_secs();

    let spawn_config_json = serde_json::to_string(&config).unwrap_or_default();

    // Create agent_run row for sub-agent
    sqlx::query(
        "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at, parent_run_id, orchestrator_chat_id, spawn_config, depth) VALUES (?, ?, 'running', 0, ?, ?, ?, ?, ?, 1)"
    )
    .bind(&sub_run_id)
    .bind(&orchestrator_chat_id)
    .bind(max_iter)
    .bind(now)
    .bind(&parent_run_id)
    .bind(&orchestrator_chat_id)
    .bind(&spawn_config_json)
    .execute(&pool).await
    .map_err(|e| e.to_string())?;

    // Build sub-agent system prompt
    let mut system_parts = vec![
        format!("You are a specialized sub-agent working on a specific task.\n\nYOUR TASK: {}", config.goal),
    ];
    if let Some(ref ctx) = config.context {
        system_parts.push(format!("\nADDITIONAL CONTEXT:\n{}", ctx));
    }

    // Load skill instructions if specified
    if let Some(ref skill_names) = config.skills {
        for name in skill_names {
            let skill_content: Option<String> = sqlx::query_scalar(
                "SELECT content FROM skills WHERE name = ? AND enabled = 1"
            ).bind(name).fetch_optional(&pool).await.unwrap_or(None);
            if let Some(content) = skill_content {
                system_parts.push(format!("\n<skill name=\"{}\">\n{}\n</skill>", name, content));
            }
        }
    }

    // Load workspace artifact contents for workspace_refs
    let sub_project_id: Option<String> = sqlx::query_scalar(
        "SELECT p.id FROM projects p JOIN chats c ON c.project_id = p.id WHERE c.id = ?"
    ).bind(&orchestrator_chat_id).fetch_optional(&pool).await.unwrap_or(None);

    if let Some(ref refs) = config.workspace_refs {
        for ref_name in refs {
            if let Ok(Some(artifact)) = crate::commands::workspace::read_artifact_impl(&pool, &orchestrator_chat_id, sub_project_id.as_deref(), ref_name).await {
                if let Some(content) = artifact.content {
                    system_parts.push(format!("\n<workspace_artifact name=\"{}\" type=\"{}\">\n{}\n</workspace_artifact>", ref_name, artifact.content_type, content));
                }
            }
        }
    }

    system_parts.push("\nYou have access to workspace tools to save your outputs. Always use workspace_write to save any code, documents, or data you produce.\nWhen you have completed your task, provide a clear summary of what you accomplished.".to_string());

    let system_prompt = Some(system_parts.join("\n"));

    // Insert initial user message scoped to sub-agent
    let user_msg_id = uni_common::generate_id();
    let _ = sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'user', ?, NULL, ?, '', 0, 0, 0.0, 0, 0, ?)"
    )
    .bind(&user_msg_id)
    .bind(&orchestrator_chat_id)
    .bind(&config.goal)
    .bind(now)
    .bind(&sub_run_id)
    .execute(&pool).await;

    // Emit sub-agent started event
    let _ = app.emit("sub-agent-started", serde_json::json!({
        "parentRunId": parent_run_id,
        "agentRunId": sub_run_id,
        "goal": config.goal,
        "model": config.model,
    }));

    // Create cancel token for sub-agent
    let cancel_token = CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(sub_run_id.clone(), cancel_token.clone());
    }

    let assistant_msg_id = uni_common::generate_id();

    // Use parent credentials (sub-agent inherits orchestrator's model by default)
    let sub_model = config.model.clone().unwrap_or_default();
    let (run_model, run_api_key, run_base_url) = if sub_model.is_empty() {
        // Inherit parent's model and credentials directly
        (parent_model.clone(), parent_api_key.clone(), parent_base_url.clone())
    } else {
        // Resolve model_id from catalog, inherit parent credentials
        let catalog_model: Option<String> = sqlx::query_scalar(
            "SELECT model_id FROM model_catalog WHERE id = ?"
        ).bind(&sub_model).fetch_optional(&pool).await.unwrap_or(None);
        let resolved_model = catalog_model.unwrap_or(sub_model);
        (resolved_model, parent_api_key.clone(), parent_base_url.clone())
    };

    if run_model.is_empty() {
        return Err("Sub-agent model could not be resolved: parent model is empty and no model specified in spawn config".to_string());
    }

    // Clone everything for the spawned task
    let pool_clone = pool.clone();
    let mcp_clone = mcp_mgr;
    let mem_clone = mem_store;
    let kb_clone = kb_store;
    let cancel_tokens_clone = cancel_tokens;
    let sub_run_id_clone = sub_run_id.clone();
    let app_clone = app;
    let chat_id_clone = orchestrator_chat_id;
    let parent_run_clone = parent_run_id;

    tokio::spawn(async move {
        agent_loop(
            app_clone.clone(),
            pool_clone.clone(),
            mcp_clone,
            mem_clone,
            kb_clone,
            chat_id_clone.clone(),
            sub_run_id_clone.clone(),
            run_model,
            Some(run_base_url),
            run_api_key,
            system_prompt,
            user_msg_id,
            assistant_msg_id,
            max_iter,
            true, // auto_mode
            cancel_token,
            cancel_tokens_clone,
            None, // temperature
            None, // max_tokens
            None, // top_p
            None, // top_k
            None, // frequency_penalty
            None, // presence_penalty
            Some(true), // supports_tool_use
            Some(sub_run_id_clone.clone()), // scope_agent_run_id — isolate messages
            None, // plan_id
            1,    // depth = 1
        ).await;

        // Emit sub-agent finished event
        let status: String = sqlx::query_scalar(
            "SELECT status FROM agent_runs WHERE id = ?"
        ).bind(&sub_run_id_clone).fetch_optional(&pool_clone).await
            .unwrap_or(None).unwrap_or_else(|| "failed".to_string());
        let iterations: i64 = sqlx::query_scalar(
            "SELECT iterations FROM agent_runs WHERE id = ?"
        ).bind(&sub_run_id_clone).fetch_optional(&pool_clone).await
            .unwrap_or(None).unwrap_or(0);
        let cost: Option<f64> = sqlx::query_scalar(
            "SELECT cost FROM agent_runs WHERE id = ?"
        ).bind(&sub_run_id_clone).fetch_optional(&pool_clone).await
            .unwrap_or(None);

        let _ = app_clone.emit("sub-agent-finished", serde_json::json!({
            "parentRunId": parent_run_clone,
            "agentRunId": sub_run_id_clone,
            "status": status,
            "iterations": iterations,
            "cost": cost,
        }));
    });

    Ok(sub_run_id)
  }) // Box::pin
}

async fn handle_memory_tool(
    app: &AppHandle,
    pool: &Pool,
    memory_store: Option<&MemoryVectorStore>,
    tool_name: &str,
    args: &serde_json::Value,
    chat_id: Option<&str>,
    project_id: Option<&str>,
) -> (String, bool) {
    match tool_name {
        "memory_save" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
            if content.is_empty() {
                return ("Error: content is required".to_string(), true);
            }
            let emb = crate::services::embedding_helper::get_embedding_provider(app).await;
            let pref = emb.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
            match crate::commands::agent_memory::create_memory_impl(
                pool,
                memory_store,
                content,
                category,
                chat_id,
                None,
                project_id,
                pref,
            )
            .await
            {
                Ok(memory) => (
                    format!("Saved to memory [{}]: {}", memory.category, memory.content),
                    false,
                ),
                Err(e) => (format!("Failed to save memory: {}", e), true),
            }
        }
        "memory_search" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as usize;
            if query.is_empty() {
                return ("Error: query is required".to_string(), true);
            }
            let emb = crate::services::embedding_helper::get_embedding_provider(app).await;
            let pref = emb.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
            match crate::commands::agent_memory::search_memories_impl(
                pool,
                memory_store,
                query,
                limit,
                project_id,
                pref,
            )
            .await
            {
                Ok(results) => {
                    if results.is_empty() {
                        ("No matching memories found. This might be new information — try kb_search (for documents) or web_search (for general knowledge).".to_string(), false)
                    } else {
                        let mut text = format!("Found {} memories:\n", results.len());
                        for r in &results {
                            text.push_str(&format!(
                                "- [{}] {} (score: {:.2})\n",
                                r.memory.category, r.memory.content, r.score
                            ));
                        }
                        (text, false)
                    }
                }
                Err(e) => (format!("Memory search failed: {}", e), true),
            }
        }
        _ => ("Unknown memory tool".to_string(), true),
    }
}

// ─── Auto-Extraction of Memories After Agent Run ────────────────

async fn extract_memories_from_run(
    app: &AppHandle,
    pool: &Pool,
    mem_store: &Option<MemoryVectorStore>,
    chat_id: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
    project_id: Option<&str>,
) {
    // Load last ~20 messages from this chat
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE chat_id = ? ORDER BY timestamp DESC LIMIT 20"
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Auto-extraction: failed to load messages: {}", e);
            return;
        }
    };

    if rows.is_empty() {
        return;
    }

    // Build conversation text (reverse to chronological order)
    let mut conversation = String::new();
    for row in rows.iter().rev() {
        let role: String = row.get("role");
        let content: String = row.get("content");
        if role == "tool" {
            continue; // skip raw tool messages for extraction
        }
        conversation.push_str(&format!("{}: {}\n\n", role, content));
    }

    // Load existing memories as dedup context
    let existing = sqlx::query("SELECT content FROM agent_memory ORDER BY updated_at DESC LIMIT 50")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let mut existing_text = String::new();
    for row in &existing {
        let content: String = row.get("content");
        existing_text.push_str(&format!("- {}\n", content));
    }

    // Load active skills for context-aware extraction
    let chat_skills = crate::commands::skills::get_chat_skills_impl(pool, chat_id)
        .await
        .unwrap_or_default();
    let skill_context = if chat_skills.is_empty() {
        String::new()
    } else {
        let names: Vec<&str> = chat_skills.iter().map(|s| s.name.as_str()).collect();
        format!("\nActive skills during this conversation: {}. Pay special attention to facts relevant to these domains.\n", names.join(", "))
    };

    let extraction_prompt = format!(
        r#"Analyze the following conversation and extract important facts, decisions, preferences, or learnings that would be useful to remember for future interactions. Return ONLY a JSON array of objects with "content" and "category" fields. Categories: fact, decision, preference, context, learning.

Rules:
- Only extract genuinely important, reusable information
- Skip ephemeral details, greetings, or task-specific minutiae
- Each fact should be self-contained and understandable out of context
- Do NOT extract anything already known (see existing memories below)
- If nothing new is worth remembering, return an empty array []
{skill_context}
Existing memories (do not duplicate):
{existing_text}

Conversation:
{conversation}

Return ONLY valid JSON array, no markdown, no explanation."#
    );

    let url = uni_llm::build_llm_url(base_url.trim_end_matches('/'));
    let headers = uni_llm::build_llm_headers(api_key);

    let client = match build_http_client(app, None).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Auto-extraction: failed to build HTTP client: {}", e);
            return;
        }
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a memory extraction assistant. Extract key facts from conversations."},
            {"role": "user", "content": extraction_prompt}
        ],
        "temperature": 0.1,
        "max_tokens": 2000
    });

    let resp = client.post(&url).headers(headers).json(&body).send().await;
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Auto-extraction: LLM request failed: {}", e);
            return;
        }
    };

    let resp_body = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Auto-extraction: failed to read response: {}", e);
            return;
        }
    };

    // Parse response: extract content from choices[0].message.content
    let resp_json: serde_json::Value = match serde_json::from_str(&resp_body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Auto-extraction: failed to parse response JSON: {}", e);
            return;
        }
    };

    let content_str = resp_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("[]");

    // Strip markdown code fences if present
    let clean = content_str
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let extracted: Vec<serde_json::Value> = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Auto-extraction: failed to parse extracted memories: {} — raw: {}", e, clean);
            return;
        }
    };

    let ms = mem_store.as_ref();
    let emb = crate::services::embedding_helper::get_embedding_provider(app).await;
    let pref = emb.as_ref().map(|b| b.as_ref() as &dyn EmbeddingProvider);
    let mut saved_count = 0;
    for item in &extracted {
        let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
        if content.is_empty() {
            continue;
        }
        match crate::commands::agent_memory::create_memory_impl(
            pool,
            ms,
            content,
            category,
            Some(chat_id),
            None,
            project_id,
            pref,
        )
        .await
        {
            Ok(_) => saved_count += 1,
            Err(e) => log::warn!("Auto-extraction: failed to save memory: {}", e),
        }
    }

    if saved_count > 0 {
        log::info!("Auto-extraction: saved {} memories from chat {}", saved_count, chat_id);
    }
}

#[cfg(test)]
mod tests {
    use super::build_tool_routing_hint;

    #[test]
    fn test_routing_hint_all_tools() {
        let hint = build_tool_routing_hint(true, true, true);
        assert!(hint.contains("memory_search"));
        assert!(hint.contains("kb_search"));
        assert!(hint.contains("web_search"));
        assert!(hint.contains("<tool_routing>"));
        assert!(hint.contains("</tool_routing>"));
    }

    #[test]
    fn test_routing_hint_no_kb() {
        let hint = build_tool_routing_hint(false, true, true);
        assert!(!hint.contains("kb_search"));
        assert!(hint.contains("web_search"));
        assert!(hint.contains("memory_search"));
    }

    #[test]
    fn test_routing_hint_no_web() {
        let hint = build_tool_routing_hint(true, false, true);
        assert!(hint.contains("kb_search"));
        assert!(!hint.contains("web_search"));
    }

    #[test]
    fn test_routing_hint_no_tools() {
        let hint = build_tool_routing_hint(false, false, false);
        assert!(hint.is_empty());
    }

    #[test]
    fn test_routing_hint_kb_and_web_has_disambiguation() {
        let hint = build_tool_routing_hint(true, true, false);
        assert!(hint.contains("kb_search first for domain questions"));
    }
}
