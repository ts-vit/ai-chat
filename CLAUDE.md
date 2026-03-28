# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**UNI AI** — a Tauri 2 desktop app (React 19 + Rust) for chatting with LLMs via OpenRouter, Ollama, and custom OpenAI-compatible providers. Two modes: **UNI Chat** (standard chat) and **UNI Assistant** (autonomous agent with tool use, memory, skills, task planning, sub-agent orchestration, shared workspace, and project workspaces). Features: streaming chat, MCP tool use, model comparison, semantic search, voice (STT/TTS), web search, image generation, agent memory, skills system, task planner, sub-agent spawning, workspace artifacts, project workspaces, message forking/branching, built-in terminal, SSH tunnel/proxy, chat export/import, snippets, folders, presets, templates, prompt library, Telegram bot gateway, scheduled tasks.

## Monorepo Structure

```
ai-chat/
├── Cargo.toml              # Rust workspace root
├── package.json             # npm workspaces root
├── CLAUDE.md
├── docs/                    # Documentation
├── crates/                  # Shared Rust crates (uni-common, uni-http, uni-llm, uni-embedding, uni-search, uni-settings, uni-ssh, uni-terminal, …)
├── packages/                # Shared npm packages
│   ├── uni-ui/              # @uni-fw/ui — React components + Mantine theme + settings modules
│   ├── uni-ssh-ui/          # @uni-fw/ssh-ui — SSH tunnel settings UI
│   └── uni-terminal-ui/     # @uni-fw/terminal-ui — Terminal panel UI (xterm.js)
└── apps/
    └── desktop/             # UNI AI Desktop (main app)
        ├── src/             # React frontend
        ├── src-tauri/       # Rust backend
        ├── package.json
        └── ...configs
```

## Commands

```bash
# From monorepo root:
npm run dev          # Full Tauri dev (frontend + backend hot-reload)
npm run build        # Production build
npm run typecheck    # TypeScript type checking
npm run test         # TypeScript tests (Vitest)
npm run test:rust    # Rust tests (cargo test --workspace)
npm run test:all     # Full suite: typecheck + vitest + cargo test

# From apps/desktop/:
npm run dev          # Same as root but direct
npm run vite:dev     # Frontend only (no Rust backend)
npm run tauri:debug  # Dev with RUST_LOG=debug

# Rust only:
cargo check --workspace      # Fast type checking
cargo test --workspace       # All Rust tests
cargo test -p ai-chat        # Desktop app tests only
```

## Testing

```bash
npm run test           # TypeScript tests (Vitest) — from root (desktop workspace)
npm run test:watch     # TypeScript tests in watch mode — from apps/desktop
npm run test -w packages/uni-ui           # @uni-fw/ui tests (Vitest + jsdom + RTL)
npm run test -w packages/uni-ssh-ui       # @uni-fw/ssh-ui tests
npm run test -w packages/uni-terminal-ui  # @uni-fw/terminal-ui tests
npm run test:rust      # Rust tests (cargo test --workspace)
npm run test:all       # Full suite: typecheck + all package vitest + cargo test

# Single test file:
npx vitest run apps/desktop/src/store/__tests__/contracts.test.ts
npx vitest run packages/uni-ui/src/__tests__/SomeComponent.test.tsx
cargo test -p uni-common test_name        # Single Rust test by name
```

Run `npm run test:all` before committing. All tests must pass.

When adding new features, include tests:
- Pure Rust functions → unit test in same file (`#[cfg(test)] mod tests`)
- DB operations → integration test in `apps/desktop/src-tauri/src/tests/`
- TypeScript utilities → vitest in `apps/desktop/src/utils/__tests__/` or `apps/desktop/src/store/__tests__/`
- New `invoke()` calls → contract test in `apps/desktop/src/store/__tests__/contracts.test.ts`
- @uni-fw/ui components/hooks → vitest in `packages/uni-ui/src/__tests__/` (jsdom env, React Testing Library)
- @uni-fw/ssh-ui, @uni-fw/terminal-ui → vitest in respective `packages/*/src/__tests__/`

## Architecture

### Frontend (React 19 + TypeScript + Vite 7)

- `apps/desktop/src/store/chatStore.ts` — Single Zustand store managing all app state. All Tauri `invoke()` calls live here.
- `apps/desktop/src/types/index.ts` — All TypeScript type definitions
- `apps/desktop/src/components/` — UI components (App.tsx is root layout; `settings/` subdirectory for settings sections)
- `apps/desktop/src/i18n/locales/{en,ru}.json` — i18n translations (both must be kept in sync)
- `apps/desktop/src/utils/` — notify, injections, tokenCount, formatDate
- `apps/desktop/src/styles/` — app.css, resize.css; duplicate `markdown.css` kept for reference (global highlight.js styles are loaded from `@uni-fw/ui` in `main.tsx`)
- `apps/desktop/src/constants/` — mcpPresets, folderColors, modes (`MODE_DEFINITIONS`), promptCategories, skillIcons
- UI: Mantine 8, icons from @tabler/icons-react

### Shared React Package: @uni-fw/ui (`packages/uni-ui/`)

Re-usable React components and theme for UNI apps. Wraps Mantine 8.

- **Theme:** `uniTheme` (brand orange palette, Inter + JetBrains Mono, component overrides), `uniCssResolver`, `brandOrange`
- **UniProvider:** Drop-in MantineProvider + Notifications + theme. Default dark color scheme.
- **MarkdownRenderer:** react-markdown + remark-gfm + rehype-highlight. Import `@uni-fw/ui/src/styles/markdown.css` for highlight.js theming.
- **Settings module** (`src/settings/`): `SettingsAdapter` interface, `TauriSettingsAdapter` (wraps Tauri invoke, snake_case→camelCase mapping), `SettingsProvider` context, `useSettings(key)` hook (value/loading/set/delete/refresh). UniProvider accepts optional `settingsAdapter` prop.
- **ConfirmModal:** Reusable confirm/cancel dialog.
- **Re-exports:** `export * from '@mantine/core'`, `@mantine/hooks`, `@mantine/notifications`.
- Apps can import Mantine components from `@uni-fw/ui` or from `@mantine/core` directly (both work).
- Desktop depends on `@uni-fw/ui` via `file:../../packages/uni-ui` in `apps/desktop/package.json` (npm workspaces link).

### Shared React Package: @uni-fw/ssh-ui (`packages/uni-ssh-ui/`)

SSH tunnel settings UI component. Uses `@uni-fw/ui` settings adapter and `@tauri-apps/api` for Tauri invoke/listen. Exports `SshTunnelSettings`.

### Shared React Package: @uni-fw/terminal-ui (`packages/uni-terminal-ui/`)

Terminal panel UI (xterm.js + tabs + PTY management). Exports `TerminalPanel` component. Uses `@tauri-apps/api` for terminal_create/write/resize/kill commands and pty-data/pty-exit events. Consumer must import `@xterm/xterm/css/xterm.css` in their entry point.

### Shared Package Pattern

Packages follow two types:
- **Type 1** (settings-only UI, e.g. `@uni-fw/ui` modules): Pure settings form using `useSettings(key)` hook. No Rust crate dependency.
- **Type 2** (UI + Rust crate, e.g. `@uni-fw/ssh-ui`, `@uni-fw/terminal-ui`): React component + corresponding Rust crate (`uni-ssh`, `uni-terminal`). Uses Tauri `invoke`/`listen` for backend communication.

All packages use: peer dependencies for React/Mantine/Tauri, `file:../../packages/...` links in desktop app, vitest + jsdom + RTL for tests.

### Shared Crates (`crates/`)

| Crate | Purpose |
|---|---|
| `uni-common` | Utilities (`generate_id`, `now_unix_secs`, `safe_truncate`, `estimate_tokens`), `UniError`, re-export of `CancellationToken` |
| `uni-http` | HTTP client builder (`build_http_client`, `build_http_client_from_params`), retry with exponential backoff (`retry_http_request`, `RetryResult`) |
| `uni-llm` | LLM types (`Message`, `Usage`, `Model`, etc.), SSE parser (`SseParser`), provider helpers (`build_llm_url`, `build_llm_headers`), non-streaming completion (`complete`, `extract_content`) |
| `uni-embedding` | Embedding trait (`EmbeddingProvider`), API providers (OpenAI `text-embedding-3-small`, Gemini `gemini-embedding`), factory (`create_embedding_provider`), `default_dimensions` |
| `uni-search` | Text chunking (`chunk_text`, `chunk_document_enriched`), reranker trait + providers (Cohere, Jina), FTS utilities (`fts_escape_query`) |
| `uni-audio` | Whisper STT (`transcribe_whisper` — OpenAI + Groq via base_url), OpenAI TTS (`speak_openai`, `openai_voice_ids`, `openai_model_ids`) |
| `uni-python` | Managed Python runtime: discovery (`discover_python`), venv management (`PythonEnvironment`), script execution via JSON-RPC (`PythonExecutor`), script registry (`ScriptRegistry`), sandbox (`Sandbox`), bridge library extraction (`ensure_bridge`) |
| `uni-converter` | Document→Markdown converter: `convert_file` (TXT, MD, HTML, CSV, PDF, RTF), `convert_url` (Jina Reader), `convert_youtube` (captions), `convert_text`, `guess_mime_type`. Python-enhanced via `uni-python`: `convert_file_with_python` (PDF/pymupdf, DOCX/mammoth, XLSX/openpyxl, PPTX/python-pptx, EPUB/ebooklib). PDF fallback: Python→Rust. |
| `uni-settings` | File-based settings store: `SettingsStore` trait, `JsonSettingsStore` (atomic JSON writes via tmp file), key constants in `keys.rs` (~100+ keys across all domains), auto-detection and masking of sensitive values (api_key, password, token, secret), prefix-filtered listing |
| `uni-ssh` | SSH tunnel with SOCKS5 proxy (russh): `SshTunnel` struct, connect/disconnect, local SOCKS5 listener, proxy URL resolution |
| `uni-terminal` | PTY terminal sessions (portable-pty): session create/write/resize/kill, pty-data/pty-exit event emission |

### Backend — Commands (`apps/desktop/src-tauri/src/commands/`)

| Module | Domain |
|---|---|
| `chat` | LLM streaming, message CRUD |
| `agent` | Agent loop, runs, step execution |
| `agent_memory` | Agent memory CRUD, search |
| `planner` | Task plan generation, execution, replan |
| `skills` | Skill CRUD, attach/detach, LLM detection |
| `workspace` | Workspace artifact CRUD, shared helpers for agent loop |
| `projects` | Project CRUD, chat-project assignment |
| `prompt_library` | Prompt library CRUD |
| `comparisons` | Side-by-side model comparison |
| `database` | Chat/message DB operations |
| `settings` | App settings read/write (legacy tauri-plugin-store) |
| `uni_settings` | Unified settings CRUD via `uni-settings` crate (get/set/delete/get_all) |
| `providers` | Custom provider management |
| `ollama` | Ollama integration |
| `embeddings` | Embedding generation |
| `search` | Hybrid search (FTS5 + vector) |
| `mcp` | MCP server management & tool calls |
| `audio` | Microphone recording; STT/TTS via `uni-audio` (Whisper API + OpenAI TTS) |
| `attachments` | File attachments |
| `folders` | Chat folder management |
| `presets` | System prompt presets |
| `snippets` | Code snippets CRUD |
| `templates` | Chat templates |
| `tokens` | Token counting (tiktoken) |
| `export_import` | Chat export/import (JSON) |
| `terminal` | PTY terminal sessions |
| `web_search` | Web search integration |
| `ssh_tunnel` | SSH tunnel/SOCKS proxy |
| `model_manager` | Model list caching |
| `injections` | Template variable resolution |
| `telegram` | Telegram bot start/stop, auth, validation |
| `scheduler` | Scheduled tasks CRUD, execution |
| `model_catalog` | Model catalog sync (OpenRouter/Ollama), catalog queries |
| `budget` | Cost tracking, budget checks, cost ledger |
| `routing` | Rule-based and LLM-based model routing |
| `knowledge_base` | Knowledge base CRUD, document indexing, chunked retrieval |
| `notebook` | Notebook CRUD, cell execution, kernel management |

### Backend — Services (`apps/desktop/src-tauri/src/services/`)

Key services (others are discoverable via file names):
- **`http_client.rs`** — Tauri wrapper: resolves active proxy (SSH tunnel → manual settings from store), then `uni_http::build_http_client`. All app HTTP MUST use `build_http_client(app, timeout)` from here, not raw `Client::new()`.
- **Retry (`uni-http`)** — `retry_http_request` with exponential backoff and Retry-After for 429. Max 3 retries. Used before first SSE chunk in chat/comparisons/llm_stream.
- **`llm_stream.rs`** — Shared LLM SSE streaming service. Uses `uni_llm::SseParser` for SSE parsing; retry via `uni-http`. Used by agent.rs; `stream_llm_call()` with configurable event prefix.
- **`embedding_helper.rs`** — Resolves `uni_embedding::EmbeddingProvider` from settings store (OpenAI / Gemini keys) and `build_http_client`; `get_embedding_dimensions()` for LanceDB table width at startup.
- Search services: `fts.rs` + `vector_store.rs` + `hybrid_search.rs` (chat search); `memory_fts.rs` + `memory_vector_store.rs` + `memory_search.rs` (agent memory). All FTS uses `uni_search::fts_escape_query`.

### Backend — Models (`apps/desktop/src-tauri/src/models/`)

LLM API types (Message, ContentBlock, ChatRequest, StreamResponse, Usage, Model, etc.) live in the **`uni-llm`** crate, not under `models/`. App-specific DB-facing types (DbChat, DbMessage, AgentRun, etc.) are in `db.rs`. Other model files map 1:1 to their domain (catalog, comparison, mcp, memory, skill, plan, workspace, project, prompt_library).

### Database

SQLite via `sqlx` (WAL mode). Migrations: `DB_MIGRATIONS` array in `lib.rs` + ALTER TABLE via `let _ = sqlx::query("ALTER TABLE ...").execute(&pool).await;` (silently ignores duplicate errors).

Key tables: `chats`, `messages`, `comparisons`, `comparison_messages`, `folders`, `presets`, `snippets`, `categories`, `chat_templates`, `mcp_servers`, `custom_providers`, `image_styles`, `agent_runs`, `agent_plans`, `agent_tasks`, `agent_memory`, `agent_memory_fts`, `chat_skills`, `skills`, `mode_settings`, `fs_audit_log`, `workspace_artifacts`, `projects`, `scheduled_tasks`, `model_catalog`, `cost_ledger`, `routing_rules`, `knowledge_bases`, `notebooks`

Notable columns: `chats.is_telegram_chat` (marks Telegram bot chat), `chats.scheduled_task_id` (links chat to scheduled task).

### Embeddings (chat search, KB, agent memory)

Semantic vectors use **`uni-embedding`** (OpenAI `text-embedding-3-small`, Gemini embedding API) via `services/embedding_helper.rs` and `build_http_client(app, timeout)`. LanceDB tables (`vector_store`, `memory_vector_store`, per-KB tables) use dimensions from settings (1536 / 768); on mismatch with an existing table, the vector table is dropped and rebuilt (FTS remains; reindex to restore semantics).

### Streaming

SSE from LLM APIs → Tauri events:
- **Chat:** `chat-stream`, `chat-stream-done`, `chat-stream-error`, `chat-stream-usage`, `chat-stream-image`, `chat-stream-tool-call`, `chat-stream-tool-result`, `chat-stream-retry`
- **Comparisons:** `comparison-stream-*` (same suffixes), two parallel `tokio::spawn`
- **Agent:** `agent-stream`, `agent-stream-done`, `agent-stream-error`, `agent-stream-usage`, `agent-stream-tool-call`, `agent-stream-tool-result`
- **Plans:** `plan-created`, `plan-status-changed`, `plan-task-updated`, `plan-task-started`, `plan-replanned`
- **Sub-agents:** `sub-agent-started`, `sub-agent-finished`, `workspace-changed`
- **Telegram:** `telegram-auth-request`, `telegram-status-changed`, `telegram-message-received`, `telegram-error`
- **Scheduler:** `scheduler-task-started`, `scheduler-task-completed`, `scheduler-task-failed`

## Agent Architecture

### Modes
Two modes: UNI Chat (standard) and UNI Assistant (agent). `MODE_DEFINITIONS` array in `constants/modes.ts`. Per-mode workspace isolation via `modeState` in Zustand store. `mode` column on `chats` and `folders` tables. In Assistant mode, projects replace folders as the grouping mechanism (see Project Workspace section).

### Agent Loop (`commands/agent.rs`)
Autonomous LLM→tool→feedback loop in `tokio::spawn`. Two execution modes: auto (runs to completion) and step-by-step (pauses after each tool call, resumed via `resume_agent_run`). Uses shared `stream_llm_call()` from `services/llm_stream.rs` with `agent-stream` event prefix. `build_agent_messages()` reconstructs OpenAI tool_calls format from flat DB messages. `scope_agent_run_id` isolates messages for parallel plan tasks. Built-in `web_search` and `web_read` tools — intercepted in agent loop alongside memory/workspace tools. `web_search` routes to configured provider (Tavily/Brave/DuckDuckGo). `web_read` uses Jina Reader with fallback, truncates to 8000 chars. Available when webSearchEnabled is true. Orchestrator guidance: direct tools (web_search, memory, workspace) for simple tasks; sub-agents only for complex parallel work. Sub-agent model resolution: inherits parent model when not specified.

### Agent Memory
Triple storage: SQLite `agent_memory` + FTS5 `agent_memory_fts` + LanceDB `memory_vectors` (dimension matches configured embedding provider). Built-in tools `memory_save` (cosine dedup > 0.85) and `memory_search` — intercepted in agent loop before MCP dispatch. Auto-injection: top-10 relevant memories into system prompt before each run. Auto-extraction: background `tokio::spawn` after run completion (temperature 0.1). Project-scoped: when chat belongs to a project, memories are saved with `project_id`; search returns both global (project_id IS NULL) and project-specific memories.

### Skills
Markdown instructions injected into agent system prompt in `<skill name="...">` XML blocks. 4 bundled + user CRUD. LLM-based auto-detection via `detect_skill_for_message` (~200 tokens, max_tokens=5). `chat_skills` join table (chat_id, skill_id, attached_by). Skill-aware memory search and extraction.

### Task Planner (`commands/planner.rs`)
LLM decomposes goal into DAG of subtasks. Tables: `agent_plans` + `agent_tasks`. Parallel execution of independent tasks via `tokio::spawn` + `futures::join_all`. LLM replan on failure (max 2). `scope_agent_run_id` for message isolation. Credentials passed once at start (not re-read from store in spawned tasks). Plans support `project_id` for project-level scoping; `get_plans_for_project` returns plans with task progress counts.

### Shared Workspace (`commands/workspace.rs`)
Project-scoped or per-chat artifact storage for agent collaboration. Table: `workspace_artifacts` (id, chat_id, project_id, name, content_type, content, file_path, created_by, updated_by, created_at, updated_at). When chat belongs to a project, artifacts are scoped to `project_id` (visible across all project chats); otherwise scoped to `chat_id`. Three built-in tools: `workspace_write` (create/update), `workspace_read` (by name), `workspace_list` — intercepted in agent loop alongside memory tools. Shared helpers (`create_artifact_impl`, `read_artifact_impl`, `list_artifacts_impl`, `update_artifact_impl`) accept `project_id: Option<&str>` and adjust SQL scoping accordingly. `workspace-changed` event emits `{ chatId, projectId }` for cross-chat UI reactivity.

### Sub-Agent Orchestration (`commands/agent.rs`)
Orchestrator (depth=0) dynamically spawns sub-agents via built-in `spawn_agent` tool. Sub-agent = full `agent_loop` with own model, skills, tools, isolated message scope. New columns on `agent_runs`: `parent_run_id`, `orchestrator_chat_id`, `spawn_config` (JSON), `depth`. Max depth = 1 (sub-agents cannot spawn sub-sub-agents). Four orchestrator-only tools: `spawn_agent`, `check_agent`, `get_agent_result`, `cancel_agent` — only available at depth==0. `spawn_sub_agent_sync` uses `Box::pin` for Send safety with recursive async calls. Credential resolution: model catalog lookup → parent credentials fallback. Sub-agent messages stored in same `messages` table, scoped by `agent_run_id` + `parent_run_id`. Workspace shared between orchestrator and sub-agents. Cascade cancel: cancelling orchestrator cascades to all running sub-agents via `AgentCancelTokens`. Sub-agent costs tracked in `cost_ledger` with source="sub_agent", aggregated into orchestrator run cost.

### Project Workspace (`commands/projects.rs`)
Long-lived container grouping chats, artifacts, memory, and plans around a shared goal. Table: `projects` (id, name, goal, status, created_at, updated_at). Status: active/completed/archived. In Assistant mode, projects replace folders as the primary grouping — sidebar shows projects with chats inside, plus free chats below. Chats link to projects via `project_id` column on `chats` table. Project goal injected into agent system prompt as `<project name="...">` XML block (between base prompt and skills). Workspace artifacts, agent memory, and plans support `project_id` for project-level scoping. Deleting a project frees its chats (sets project_id=NULL) and deletes project-level artifacts/memory/plans. `ProjectDashboardPage` — full page aggregating project goal, chats, artifacts, memory, and plans with inline editing. Three navigation entry points: project name click in sidebar, toolbar icon (IconLayoutBoard), context menu.

### Telegram Gateway (`services/telegram_bot.rs`)
Built-in Telegram bot for remote access to UNI Assistant. Long polling (no external server needed). Single-user, authorized by Telegram user ID with auto-discovery on first contact. Messages processed through existing `agent_loop()` — memory, skills, web search, MCP tools all work. TelegramRunNotifier (oneshot channel) for awaiting agent completion. Context window monitoring at 80% with warning injection. `/new` for fresh sessions, `/status` for info. Queued messages processed on startup (last 5). Configurable model per Telegram in settings. HTML parse mode for responses. Message splitting at 4096 chars (UTF-8 safe). Telegram settings stored in settings.json store (telegramBotToken, telegramUserId, telegramEnabled, telegramAutoStart, telegramModel).

### Scheduler (`services/scheduler.rs`)
Cron-based task scheduler. Table: `scheduled_tasks` (id, name, prompt, cron_expression, enabled, model, skill_id, project_id, deliver_telegram, deliver_desktop_notification, last_run_at, next_run_at, last_run_status, run_count). Background tokio task checks every 30s for due tasks. Each task = prompt sent to agent_loop. Results delivered via Telegram (if connected) and/or desktop notification. Context rotation when chat exceeds 100 messages. Uses `cron` crate for expression parsing. Reuses TelegramRunNotifier for agent completion. Auto-starts with app. `SchedulerPage` + `CreateScheduledTaskModal` with cron presets.

### Cost Tracking, Routing & Budget (`commands/budget.rs`, `commands/routing.rs`, `commands/model_catalog.rs`)
Model catalog synced from OpenRouter/Ollama into `model_catalog` table. `cost_ledger` records per-call costs (prompt/completion tokens × catalog pricing). `routing_rules` map task categories to models (rule-based or LLM-based via `llm_route_task`). Budget enforcement via `check_budget()` before LLM calls — emits `budget-exceeded` event and pauses agent/plan with status `paused_budget`. `agent_runs` tracks `prompt_tokens`, `completion_tokens`, `cost`, `assigned_model`. Sub-agent costs aggregated into orchestrator run.

## Critical Conventions

### Tauri 2 Command Pattern
Commands MUST use individual parameters, NOT struct wrappers:
```rust
// CORRECT
#[tauri::command]
pub async fn create_item(pool: State<'_, Pool>, name: String, value: String) -> Result<(), String> {}
```
```typescript
// Frontend: camelCase params — Tauri 2 auto-converts to snake_case
await invoke("create_item", { name: "foo", value: "bar" });
```
Never use `#[serde(rename)]` on Tauri command parameters.

### UUID generation
Use `uni_common::generate_id()` (or `generate_id` via `use`). Do not inline `Uuid::new_v4().to_string()` in new code.

### Timestamps
Store as Unix **seconds** (not ms) in SQLite. Frontend uses `toUnixSeconds()` for normalization. In Rust, use `uni_common::now_unix_secs()` for current wall time; do not inline `SystemTime::now().duration_since(UNIX_EPOCH)...` for that purpose. Keep `chrono::Utc::now().timestamp()` where chrono is already used; do not replace file `metadata.modified()` / `created()` conversions with `now_unix_secs()`.

### Text truncation
Use `uni_common::safe_truncate()` for UTF-8–safe byte limits (`&str`), and `uni_common::safe_truncate_chars()` when truncating by character count with a `...` suffix.

### i18n
Use `useTranslation()` in components, `i18n.t()` in store/utils. Update both `en.json` and `ru.json`.

### HTTP Client
In shared crates, use `uni_http::build_http_client()`. In desktop app code, use `crate::services::http_client::build_http_client(app, timeout)` which wraps uni-http with Tauri proxy/SSH settings. Never create `reqwest::Client::new()` directly.

### DB Migrations
New tables: `CREATE TABLE IF NOT EXISTS`. New columns via ALTER TABLE: `let _ = sqlx::query("ALTER TABLE x ADD COLUMN y ...").execute(&pool).await;` — silently ignores errors if column already exists. All migrations run in `lib.rs` setup.

### Notifications
`notify.error()` — 5 sec, `notify.success()` — 3 sec, `notify.warning()` — 4 sec, `notify.info()` — 3 sec. Supports `id` param for upsert.

### Image Generation
- Detection: `is_image_generation_model()` in `commands/chat.rs`
- API: `modalities: ["text", "image"]` for image models
- Config: `image_config` (size, quality, n) spread to top-level request body
- Styles: prompt suffix appended to user text; original text stored in DB

### Contextual Injections
`{{date}}`, `{{clipboard}}`, `{{file:path}}` in messages:
- User messages: resolved BEFORE saving to DB
- System prompts: template stays in DB, resolved only for API calls

### Rust Error Handling
Use `map_err(|e| e.to_string())?` for error propagation in Tauri commands. Log with `eprintln!` before returning errors. CancellationToken for stopping generation.

### block_on Prohibition
NEVER call `block_on` inside async tokio context. All blocking calls must be async.

### Embedding API usage
Use `embed_documents` for indexing stored text and `embed_query` for search queries (`uni_embedding::EmbeddingProvider`). Providers apply the correct API semantics internally.

### UTF-8 Safe Truncation
Always use `is_char_boundary()` when truncating strings — Russian/multilingual text will panic on arbitrary byte offsets.

### Credentials in Spawned Tasks
In `tokio::spawn` contexts (agent loop, plan tasks), pass `api_key`/`base_url` as parameters. Do NOT re-read from Tauri managed state — it may be unavailable in spawned tasks.

### Recursive Async and Box::pin
When agent_loop spawns sub-agents that themselves call agent_loop, use `Box::pin` for the recursive future to satisfy Rust's Send requirements. This pattern is used in `spawn_sub_agent_sync`.

### Sub-Agent Depth Limit
The `depth` parameter controls available tools: depth=0 gets orchestrator tools (spawn/check/get_result/cancel), depth≥1 does not. This prevents infinite recursion. Always pass depth when calling agent_loop or agent_loop_resume.

### Regex with /g Flag
JavaScript regex with the `/g` flag is stateful (`lastIndex`). Always create inside the function body, never at module level.

### Style Guide
See `docs/STYLEGUIDE.md` for visual conventions. Key points: brand color `#D4854A`, Mantine CSS variables only (no hardcoded colors), `@tabler/icons-react` with `stroke={1.5}`.

### Store Architecture
Single Zustand store decomposed into domain slices in `apps/desktop/src/store/slices/`:
- `chatSlice.ts` (~840 lines) — Core chat, messaging, folders, projects, attachments, TTS
- `agentSlice.ts` (~900 lines) — Agent loop, memory, skills, workspace, plans, scheduler
- `settingsSlice.ts` (~480 lines) — Settings, providers, models, Ollama, MCP, filesystem
- `uiSlice.ts` (~560 lines) — View navigation, modes, comparisons, telegram
- `snippetsSlice.ts` (~320 lines) — Snippets, presets, categories, templates, prompt library
- `helpers.ts` — Shared helpers (resolveProvider, DB mappers, reloadChatMessages)
- `chatStore.ts` (~290 lines) — Hub: combines slices, exports `useChatStore`
All components import from `useChatStore` unchanged. Cross-slice access via `get()` returning full `StoreState`.

### Communication
Russian for discussion, English for code comments. Plan first → approval → implementation. Run `npm run typecheck` before committing.
