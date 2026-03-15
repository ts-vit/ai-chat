use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::skill::Skill;

type Pool = sqlx::SqlitePool;

fn unix_now() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .map(|d| d.as_secs() as i64)
}

fn map_skill_row(row: sqlx::sqlite::SqliteRow) -> Skill {
    Skill {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        icon: row.get("icon"),
        content: row.get("content"),
        trigger_description: row.get("trigger_description"),
        required_tools: row.get("required_tools"),
        is_builtin: row.get::<i32, _>("is_builtin") != 0,
        enabled: row.get::<i32, _>("enabled") != 0,
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[tauri::command]
pub async fn list_skills(pool: State<'_, Pool>) -> Result<Vec<Skill>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at FROM skills ORDER BY sort_order ASC, name ASC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_skill_row).collect())
}

#[tauri::command]
pub async fn create_skill(
    pool: State<'_, Pool>,
    name: String,
    description: String,
    icon: String,
    content: String,
    trigger_description: String,
    required_tools: String,
) -> Result<Skill, String> {
    let id = Uuid::new_v4().to_string();
    let now = unix_now()?;
    let max_order = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM skills",
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO skills (id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .bind(&icon)
    .bind(&content)
    .bind(&trigger_description)
    .bind(&required_tools)
    .bind(max_order)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(Skill {
        id,
        name,
        description,
        icon,
        content,
        trigger_description,
        required_tools,
        is_builtin: false,
        enabled: true,
        sort_order: max_order,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_skill(
    pool: State<'_, Pool>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    content: Option<String>,
    trigger_description: Option<String>,
    required_tools: Option<String>,
    enabled: Option<bool>,
    sort_order: Option<i64>,
) -> Result<(), String> {
    let now = unix_now()?;
    let mut sets = vec!["updated_at = ?".to_string()];
    let mut binds: Vec<String> = vec![now.to_string()];

    if let Some(v) = name {
        sets.push("name = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = description {
        sets.push("description = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = icon {
        sets.push("icon = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = content {
        sets.push("content = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = trigger_description {
        sets.push("trigger_description = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = required_tools {
        sets.push("required_tools = ?".to_string());
        binds.push(v);
    }
    if let Some(v) = enabled {
        sets.push("enabled = ?".to_string());
        binds.push((v as i32).to_string());
    }
    if let Some(v) = sort_order {
        sets.push("sort_order = ?".to_string());
        binds.push(v.to_string());
    }

    let sql = format!("UPDATE skills SET {} WHERE id = ?", sets.join(", "));
    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }
    query = query.bind(&id);
    query.execute(pool.inner()).await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_skill(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    let is_builtin = sqlx::query_scalar::<_, i32>(
        "SELECT is_builtin FROM skills WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if is_builtin == Some(1) {
        return Err("Cannot delete built-in skill".to_string());
    }

    sqlx::query("DELETE FROM chat_skills WHERE skill_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM skills WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_chat_skills(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<Vec<Skill>, String> {
    get_chat_skills_impl(pool.inner(), &chat_id).await
}

pub async fn get_chat_skills_impl(
    pool: &sqlx::SqlitePool,
    chat_id: &str,
) -> Result<Vec<Skill>, String> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.description, s.icon, s.content, s.trigger_description, s.required_tools, s.is_builtin, s.enabled, s.sort_order, s.created_at, s.updated_at FROM skills s INNER JOIN chat_skills cs ON cs.skill_id = s.id WHERE cs.chat_id = ? ORDER BY s.sort_order ASC",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_skill_row).collect())
}

#[tauri::command]
pub async fn attach_skill_to_chat(
    pool: State<'_, Pool>,
    chat_id: String,
    skill_id: String,
    attached_by: Option<String>,
) -> Result<(), String> {
    let now = unix_now()?;
    let by = attached_by.unwrap_or_else(|| "manual".to_string());
    sqlx::query(
        "INSERT OR IGNORE INTO chat_skills (chat_id, skill_id, attached_by, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&chat_id)
    .bind(&skill_id)
    .bind(&by)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn detach_skill_from_chat(
    pool: State<'_, Pool>,
    chat_id: String,
    skill_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM chat_skills WHERE chat_id = ? AND skill_id = ?")
        .bind(&chat_id)
        .bind(&skill_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn detect_skill_for_message(
    pool: State<'_, Pool>,
    app: tauri::AppHandle,
    api_key: String,
    base_url: String,
    model: String,
    message_text: String,
) -> Result<Option<Skill>, String> {
    detect_skill_impl(pool.inner(), &app, &api_key, &base_url, &model, &message_text).await
}

pub async fn detect_skill_impl(
    pool: &sqlx::SqlitePool,
    app: &tauri::AppHandle,
    api_key: &str,
    base_url: &str,
    model: &str,
    message_text: &str,
) -> Result<Option<Skill>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, icon, content, trigger_description, required_tools, is_builtin, enabled, sort_order, created_at, updated_at FROM skills WHERE enabled = 1 AND trigger_description != '' ORDER BY sort_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let skills: Vec<Skill> = rows.into_iter().map(map_skill_row).collect();
    if skills.is_empty() {
        return Ok(None);
    }

    let skills_list: String = skills
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {} — {}", i + 1, s.name, s.trigger_description))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        r#"You are a skill classifier. Given a user message, determine which expert skill (if any) is most appropriate.

Available skills:
{skills_list}

User message: "{message_text}"

Rules:
- If the message clearly matches one skill, respond with ONLY the skill number (e.g. "1")
- If the message doesn't clearly match any skill, respond with "0"
- If unsure, respond with "0"
- Respond with ONLY a single number, nothing else

Your answer:"#
    );

    let base = if base_url.is_empty() {
        "https://openrouter.ai/api/v1"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/chat/completions", base);

    let client = crate::services::http_client::build_http_client(app, None)
        .await
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 5,
        "temperature": 0
    });

    let mut req = client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let resp_json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let answer = resp_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("0");

    // Extract first number from response
    let num: usize = answer
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);

    if num == 0 || num > skills.len() {
        return Ok(None);
    }

    Ok(Some(skills[num - 1].clone()))
}

// Bundled skills data: (id, name, description, icon, content, trigger_description, required_tools)
pub const BUILTIN_SKILLS_DATA: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    (
        "builtin-skill-programmer",
        "Programmer",
        "Code writing, debugging, refactoring, architecture, and code review",
        "code",
        r#"You are an expert programmer. Follow these principles:

## Code Quality
- Write clean, readable, well-structured code
- Follow the conventions of the language being used
- Add meaningful comments for complex logic only — don't over-comment obvious code
- Use descriptive variable and function names
- Handle errors properly — never silently ignore errors

## Architecture
- Prefer simple solutions over clever ones
- Follow SOLID principles where applicable
- Design for testability and maintainability
- Consider edge cases and error scenarios

## When Helping the User
- Ask clarifying questions if the requirements are ambiguous
- Explain your approach before writing code
- If refactoring, explain what changed and why
- Suggest improvements but respect the user's existing patterns
- When debugging, explain the root cause, not just the fix"#,
        "When the user asks to write, debug, refactor, review, or architect code in any programming language. Also when discussing algorithms, data structures, APIs, databases, or software design patterns.",
        "[\"filesystem\"]",
    ),
    (
        "builtin-skill-writer",
        "Writer",
        "Writing, editing, articles, documentation, and content creation",
        "pencil",
        r#"You are an expert writer and editor. Follow these principles:

## Writing Quality
- Write clearly and concisely — remove unnecessary words
- Use active voice by default
- Vary sentence length for rhythm
- Structure content with clear logical flow
- Match tone to the audience and purpose

## Editing
- Preserve the author's voice when editing
- Fix grammar, punctuation, and spelling
- Improve clarity without changing meaning
- Suggest structural improvements when relevant

## When Helping the User
- Ask about target audience and purpose if not clear
- Provide multiple options for headlines/titles when asked
- Explain significant changes when editing
- Respect style guides if mentioned"#,
        "When the user asks to write, edit, proofread, or improve text — articles, blog posts, documentation, emails, reports, creative writing, copy, or any written content.",
        "[]",
    ),
    (
        "builtin-skill-analyst",
        "Analyst",
        "Data analysis, research, comparisons, and structured thinking",
        "chart-bar",
        r#"You are an expert analyst. Follow these principles:

## Analysis
- Start with understanding the question and what decision it informs
- Gather relevant data before forming conclusions
- Consider multiple perspectives and counterarguments
- Distinguish between facts, inferences, and opinions
- Quantify when possible — use numbers, not vague qualifiers

## Research
- Use web search to find current, reliable data
- Cross-reference multiple sources
- Cite sources and note their reliability
- Acknowledge gaps in available information

## When Helping the User
- Structure analysis clearly: problem → data → analysis → conclusion → recommendation
- Present findings objectively before giving recommendations
- Use tables and comparisons for multi-option evaluations
- Flag assumptions and uncertainties explicitly"#,
        "When the user asks to analyze data, research a topic, compare options, make a decision, evaluate pros and cons, create a report, or think through a problem systematically.",
        "[\"web_search\"]",
    ),
    (
        "builtin-skill-sysadmin",
        "System Administrator",
        "Server management, deployment, configuration, DevOps, and CLI tools",
        "terminal-2",
        r#"You are an expert system administrator and DevOps engineer. Follow these principles:

## Safety First
- Always warn before destructive operations
- Suggest backups before major changes
- Use --dry-run flags when available
- Prefer reversible actions over irreversible ones

## Best Practices
- Use infrastructure as code when possible
- Follow the principle of least privilege
- Document configuration changes
- Consider security implications of every action

## When Helping the User
- Explain what commands do before suggesting them
- Provide rollback steps for risky operations
- Suggest monitoring after changes
- Ask about the environment (OS, cloud provider) if not specified"#,
        "When the user asks about server setup, deployment, Docker, CI/CD, shell commands, system configuration, networking, cloud services, monitoring, or any DevOps/sysadmin task.",
        "[\"filesystem\"]",
    ),
];
