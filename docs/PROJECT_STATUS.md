# Project Status

## Phase 3: Multi-Model Routing + Cost Tracking + Budget ✅

Implemented as specified in the plan.

### Backend (Rust/Tauri)

- **Model catalog:** Table `model_catalog`, sync from OpenRouter and Ollama, Tauri commands `sync_model_catalog`, `get_model_catalog`, `update_model_catalog_entry`. Cost helper and `catalog_id()` in `commands/budget.rs`.
- **Cost tracking:** Table `cost_ledger`; `write_cost_ledger` and `compute_cost_from_catalog`. `agent_runs` has `prompt_tokens`, `completion_tokens`, `cost`, `assigned_model`; `messages.cost`; `finish_run` updates run cost from ledger. Chat: `update_message_usage` accepts optional `chat_id`, `model_id`, `provider` and writes to cost_ledger.
- **Routing:** Table `routing_rules` with seed defaults. Rule-based `get_model_for_category`, LLM-based `llm_route_task` (writes routing cost to ledger). Planner: task `category` and `assigned_model`; credentials via `resolve_catalog_id_to_credentials`. Agent: routing in `send_agent_message`, `assigned_model` on run.
- **Budget:** `check_budget()` before LLM calls; `budget-exceeded` event; `get_budget_status_command` for UI. Plan and global (daily/monthly) limits from store. Agent/plan pause with status `paused_budget`.

### Frontend

- **Settings:** Routing section (strategy: rules/LLM, sync models, last sync); Budget section (per-plan and global toggles, limits, period). New keys in AppSettings and store.
- **PlanPanel:** Task card shows assigned model badge when `assigned_model` is set.
- **Chat:** `update_message_usage` called with `chatId`, `modelId`, `provider` for cost_ledger.

### i18n

- `routing.*` and `budget.*` keys added in `en.json` and `ru.json`.
