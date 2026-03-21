# Шпаргалка по Claude Code

## Установка и запуск

```bash
# Установка
npm install -g @anthropic/claude-code

# Запуск интерактивной сессии
claude

# Запуск с начальным промптом
claude "объясни этот проект"

# Проверка версии
claude --version

# Обновление
claude update

# Проверка здоровья установки
claude /doctor
```

---

## Основные команды терминала

| Команда | Описание |
|---------|----------|
| `claude` | Интерактивная сессия (REPL) |
| `claude "промпт"` | Разовая команда |
| `claude -p "промпт"` | Print-режим: выполнить и выйти (для скриптов) |
| `claude -c` | Продолжить последнюю сессию |
| `claude -c -p "промпт"` | Продолжить неинтерактивно |
| `claude --resume <id>` | Возобновить сессию по ID |
| `claude --image path/to/img.png "что на картинке?"` | Анализ изображения |

---

## Slash-команды (внутри сессии)

| Команда | Описание |
|---------|----------|
| `/help` | Список всех команд |
| `/exit` или `Ctrl+D` | Выйти |
| `/clear` | Очистить историю разговора |
| `/compact [инструкция]` | Сжать контекст (сохранить только важное) |
| `/model` | Переключить модель (Sonnet / Haiku / Opus) |
| `/config` | Открыть настройки |
| `/cost` или `/cos` | Показать стоимость текущей сессии |
| `/review` | Запросить ревью кода |
| `/rewind` | Откатить последнее действие |
| `/doctor` | Диагностика установки |
| `/mcp` | Управление MCP-серверами |
| `/add-dir` | Добавить рабочую директорию |
| `/ide` | Управление интеграцией с IDE |

---

## Горячие клавиши

| Клавиша | Действие |
|---------|----------|
| `Ctrl+C` | Отменить текущую операцию |
| `Ctrl+D` | Выйти из Claude Code |
| `Tab` | Автодополнение |
| `↑ / ↓` | История команд |
| `Esc + Esc` | Откатить последнее действие |
| `Shift+Tab+Tab` | Plan Mode (режим планирования) |

---

## Режимы работы

### Plan Mode (Shift + Tab + Tab)
Claude сначала составляет план, а потом реализует. Используй для сложных задач.

### Print Mode (-p)
Неинтерактивный — для скриптов и автоматизации:
```bash
claude -p "проанализируй кодовую базу" --output-format json > analysis.json
cat error.log | claude -p "найди причину ошибки"
git log --oneline | claude -p "опиши эти коммиты"
```

### YOLO Mode
Без запроса разрешений (опасно!):
```bash
claude --dangerously-skip-permissions
```

---

## Управление контекстом

```bash
# Сжать разговор (сохранить ключевые моменты)
/compact сохрани только архитектурные решения

# Очистить контекст полностью
/clear

# Ограничить число шагов
claude -p --max-turns 5 "конкретный вопрос"

# Продолжить предыдущую сессию
claude -c

# Возобновить конкретную сессию
claude --resume <session_id>
```

**Совет:** Используй `/compact` между задачами чтобы не раздувать контекст (лимит 200K токенов).

---

## Модели

| Модель | Когда использовать |
|--------|-------------------|
| **Sonnet** | По умолчанию. Хорош для большинства задач |
| **Haiku** | Быстрый и дешёвый. Для простых задач |
| **Opus** | Самый мощный. Для сложных архитектурных задач |

```bash
# Переключить в сессии
/model

# Указать при запуске
claude --model claude-sonnet-4-20250514
```

---

## Права доступа (Permissions)

```bash
# Разрешить конкретные инструменты
claude --allowedTools "Bash(git:*)" "Write" "Read"

# Запретить опасные команды
claude --disallowedTools "Bash(rm:*)" "Bash(sudo:*)"

# Комбинация
claude --allowedTools "Bash(git:*)" "Write" "Read" \
       --disallowedTools "Bash(rm:*)" "Bash(sudo:*)"
```

---

## CLAUDE.md — контекст проекта

Claude Code автоматически читает файлы `CLAUDE.md` для понимания проекта.

| Расположение | Область действия |
|-------------|-----------------|
| `CLAUDE.md` (корень проекта) | Текущий проект |
| `~/.claude/CLAUDE.md` | Все проекты (глобальный) |

Что писать в CLAUDE.md:
```markdown
# Проект

## Команды
npm run dev         # Запуск в dev-режиме
npm run typecheck   # Проверка типов

## Архитектура
- Frontend: React + TypeScript
- Backend: Rust + Tauri 2
- БД: SQLite через sqlx

## Конвенции
- Invoke параметры: camelCase
- Timestamps: Unix seconds
- Стиль: Mantine CSS-переменные
```

---

## Кастомные команды

Создай файлы в `.claude/commands/` (проектные) или `~/.claude/commands/` (личные).

Пример `.claude/commands/review.md`:
```markdown
---
description: Ревью текущих изменений
---
Проведи ревью всех изменённых файлов:
1. Проверь типы
2. Найди потенциальные баги
3. Предложи улучшения
```

Вызов: `/review`

---

## MCP-серверы

```bash
# Добавить MCP-сервер
claude mcp add --transport stdio my-server -- npx -y @some/mcp-server

# С переменными окружения
claude mcp add --transport stdio github \
  --env GITHUB_TOKEN=ghp_xxx \
  -- npx -y @modelcontextprotocol/server-github

# Управление в сессии
/mcp
```

---

## Хуки (автоматические действия)

В `.claude/settings.json`:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write(*.py)",
        "hooks": [
          {
            "type": "command",
            "command": "python -m black $file"
          }
        ]
      }
    ]
  }
}
```

Хуки выполняются **всегда** — Claude не может их пропустить.

---

## Пайпинг и автоматизация

```bash
# Анализ логов
cat error.log | claude -p "найди причину ошибки"

# Анализ diff
git diff | claude -p "опиши изменения"

# Цепочка вызовов
claude -p "проанализируй" --output-format json > result.json
claude -p "сгенерируй тесты" --max-turns 3 > tests.txt

# Многосессионный скрипт
session_id=$(claude -p "начни ревью" --output-format json | jq -r '.session_id')
claude --resume "$session_id" -p "проверь безопасность"
claude --resume "$session_id" -p "сделай итог"
```

---

## Полезные паттерны

### Работа с несколькими директориями
```bash
claude --add-dir ../frontend ../backend ../shared
```

### Git Worktrees (параллельная работа)
```bash
git worktree add ../feature-auth -b feature/auth main
cd ../feature-auth && claude
# Основная работа не прерывается
```

### Стоимость сессии
```bash
/cos   # Показать стоимость и длительность
```

### Отладка
```bash
claude --verbose --debug
```

---

## Конфигурация (.claude/settings.json)

```json
{
  "model": "claude-sonnet-4-20250514",
  "permissions": {
    "allowedTools": ["Read", "Write", "Bash(git *)"],
    "deny": ["Read(./.env)", "Read(./.env.*)"]
  }
}
```

---

## Советы

1. **Начинай с плана** — используй Plan Mode для сложных задач
2. **Не раздувай контекст** — `/compact` между задачами
3. **Продолжай сессии** — `claude -c` вместо нового старта
4. **CLAUDE.md обязателен** — экономит токены на объяснение проекта
5. **Haiku для мелочей** — не трать Opus на простые вопросы
6. **Хуки для рутины** — форматирование, линтинг, проверки
7. **Кастомные команды** — автоматизируй повторяющиеся задачи
8. **Следи за /cos** — стоимость может расти быстро
