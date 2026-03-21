use sqlx::Row;
use tauri::State;

use crate::models::prompt_library::PromptLibraryItem;

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn get_prompt_library(
    pool: State<'_, Pool>,
    category: Option<String>,
    search: Option<String>,
    language: Option<String>,
) -> Result<Vec<PromptLibraryItem>, String> {
    let mut sql = String::from(
        "SELECT id, title, description, content, category, is_builtin, language, created_at, updated_at FROM prompt_library WHERE 1=1",
    );
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref cat) = category {
        if !cat.is_empty() {
            sql.push_str(" AND category = ?");
            binds.push(cat.clone());
        }
    }
    if let Some(ref lang) = language {
        if !lang.is_empty() {
            sql.push_str(" AND language = ?");
            binds.push(lang.clone());
        }
    }
    if let Some(ref q) = search {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (title LIKE ? OR description LIKE ? OR content LIKE ?)");
            let like = format!("%{}%", q);
            binds.push(like.clone());
            binds.push(like.clone());
            binds.push(like);
        }
    }
    sql.push_str(" ORDER BY is_builtin DESC, title ASC");

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    let rows = query
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let items = rows
        .into_iter()
        .map(|row| PromptLibraryItem {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            content: row.get("content"),
            category: row.get("category"),
            is_builtin: row.get("is_builtin"),
            language: row.get("language"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(items)
}

#[tauri::command]
pub async fn create_prompt_library_item(
    pool: State<'_, Pool>,
    title: String,
    description: String,
    content: String,
    category: String,
    language: Option<String>,
) -> Result<PromptLibraryItem, String> {
    let id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();
    let lang = language.unwrap_or_else(|| "en".to_string());

    sqlx::query(
        "INSERT INTO prompt_library (id, title, description, content, category, is_builtin, language, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(&description)
    .bind(&content)
    .bind(&category)
    .bind(&lang)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(PromptLibraryItem {
        id,
        title,
        description,
        content,
        category,
        is_builtin: false,
        language: lang,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_prompt_library_item(
    pool: State<'_, Pool>,
    id: String,
    title: String,
    description: String,
    content: String,
    category: String,
) -> Result<(), String> {
    let is_builtin: bool = sqlx::query_scalar("SELECT is_builtin FROM prompt_library WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    if is_builtin {
        return Err("Cannot edit built-in prompts".to_string());
    }

    let now = uni_common::now_unix_secs();
    sqlx::query(
        "UPDATE prompt_library SET title = ?, description = ?, content = ?, category = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&title)
    .bind(&description)
    .bind(&content)
    .bind(&category)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_prompt_library_item(
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    let is_builtin: bool = sqlx::query_scalar("SELECT is_builtin FROM prompt_library WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Prompt not found".to_string())?;

    if is_builtin {
        return Err("Cannot delete built-in prompts".to_string());
    }

    sqlx::query("DELETE FROM prompt_library WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// (id, title, description, content, category, language)
pub const BUILTIN_PROMPTS_DATA: &[(&str, &str, &str, &str, &str, &str)] = &[
    // Coding - EN
    ("builtin-code-review-en", "Code Review", "Review code for bugs, performance, and best practices", "Review this code for bugs, performance issues, and best practices. Suggest improvements:\n\n{code}", "coding", "en"),
    ("builtin-explain-code-en", "Explain Code", "Get a step-by-step explanation of code", "Explain what this code does step by step:\n\n{code}", "coding", "en"),
    ("builtin-write-tests-en", "Write Tests", "Generate unit tests for your code", "Write comprehensive unit tests for this code using {test_framework}:\n\n{code}", "coding", "en"),
    ("builtin-refactor-en", "Refactor Code", "Improve code structure and readability", "Refactor this code to improve readability, maintainability, and follow best practices:\n\n{code}", "coding", "en"),
    // Writing - EN
    ("builtin-rewrite-en", "Rewrite Text", "Rewrite text in a different style", "Rewrite the following text to be more {style}:\n\n{text}", "writing", "en"),
    ("builtin-summarize-en", "Summarize", "Create a concise summary of text", "Summarize the following text in {length}:\n\n{text}", "writing", "en"),
    ("builtin-proofread-en", "Proofread", "Check text for grammar and style issues", "Proofread the following text. Fix grammar, spelling, and punctuation errors. Suggest style improvements:\n\n{text}", "writing", "en"),
    // Analysis - EN
    ("builtin-pros-cons-en", "Pros and Cons", "Analyze advantages and disadvantages", "Analyze the pros and cons of {topic}. Present them in a balanced, structured format.", "analysis", "en"),
    ("builtin-compare-en", "Compare Options", "Compare two or more options side by side", "Compare {option_1} vs {option_2}. Include key differences, strengths, weaknesses, and a recommendation.", "analysis", "en"),
    // Creative - EN
    ("builtin-story-en", "Story Starter", "Generate a creative story", "Write a short story about {theme} in the style of {genre}.", "creative", "en"),
    ("builtin-brainstorm-en", "Brainstorm Ideas", "Generate creative ideas on a topic", "Brainstorm 10 creative ideas for {topic}. For each idea, provide a brief description and potential impact.", "creative", "en"),
    // Productivity - EN
    ("builtin-meeting-notes-en", "Meeting Notes", "Organize notes into action items", "Organize these meeting notes into action items with owners and deadlines:\n\n{notes}", "productivity", "en"),
    ("builtin-email-en", "Draft Email", "Write a professional email", "Write a professional email about {subject} to {recipient}. Tone: {tone}.", "productivity", "en"),
    // Learning - EN
    ("builtin-eli5-en", "Explain Like I'm 5", "Get a simple explanation of a concept", "Explain {concept} in simple terms that a beginner would understand. Use analogies and examples.", "learning", "en"),
    ("builtin-study-plan-en", "Study Plan", "Create a structured learning plan", "Create a structured study plan for learning {subject} over {duration}. Include resources, milestones, and practice exercises.", "learning", "en"),
    // General - EN
    ("builtin-translate-en", "Translate", "Translate text to another language", "Translate the following text to {language}:\n\n{text}", "general", "en"),
    ("builtin-improve-prompt-en", "Improve Prompt", "Enhance a prompt for better AI responses", "Improve this prompt to get better, more detailed responses from an AI assistant:\n\n{prompt}", "general", "en"),
    // Coding - RU
    ("builtin-code-review-ru", "Ревью кода", "Проверка кода на баги, производительность и лучшие практики", "Проведи ревью этого кода на наличие багов, проблем с производительностью и соответствие лучшим практикам. Предложи улучшения:\n\n{code}", "coding", "ru"),
    ("builtin-explain-code-ru", "Объяснить код", "Пошаговое объяснение кода", "Объясни, что делает этот код, шаг за шагом:\n\n{code}", "coding", "ru"),
    ("builtin-write-tests-ru", "Написать тесты", "Сгенерировать юнит-тесты для кода", "Напиши подробные юнит-тесты для этого кода, используя {test_framework}:\n\n{code}", "coding", "ru"),
    ("builtin-refactor-ru", "Рефакторинг кода", "Улучшить структуру и читаемость кода", "Отрефактори этот код для улучшения читаемости, поддерживаемости и соответствия лучшим практикам:\n\n{code}", "coding", "ru"),
    // Writing - RU
    ("builtin-rewrite-ru", "Переписать текст", "Переписать текст в другом стиле", "Перепиши следующий текст, сделав его более {style}:\n\n{text}", "writing", "ru"),
    ("builtin-summarize-ru", "Резюмировать", "Создать краткое изложение текста", "Резюмируй следующий текст в {length}:\n\n{text}", "writing", "ru"),
    ("builtin-proofread-ru", "Корректура", "Проверить текст на грамматику и стиль", "Проведи корректуру следующего текста. Исправь грамматические, орфографические и пунктуационные ошибки. Предложи стилистические улучшения:\n\n{text}", "writing", "ru"),
    // Analysis - RU
    ("builtin-pros-cons-ru", "За и против", "Анализ преимуществ и недостатков", "Проанализируй плюсы и минусы {topic}. Представь их в сбалансированном, структурированном формате.", "analysis", "ru"),
    ("builtin-compare-ru", "Сравнить варианты", "Сравнить два или более варианта", "Сравни {option_1} и {option_2}. Укажи ключевые различия, сильные и слабые стороны, и дай рекомендацию.", "analysis", "ru"),
    // Creative - RU
    ("builtin-story-ru", "Начало истории", "Сгенерировать творческую историю", "Напиши короткий рассказ о {theme} в стиле {genre}.", "creative", "ru"),
    ("builtin-brainstorm-ru", "Мозговой штурм", "Сгенерировать творческие идеи по теме", "Придумай 10 творческих идей для {topic}. Для каждой идеи дай краткое описание и потенциальное влияние.", "creative", "ru"),
    // Productivity - RU
    ("builtin-meeting-notes-ru", "Заметки со встречи", "Организовать заметки в задачи", "Организуй эти заметки со встречи в список задач с ответственными и дедлайнами:\n\n{notes}", "productivity", "ru"),
    ("builtin-email-ru", "Написать письмо", "Написать деловое письмо", "Напиши деловое письмо на тему {subject} для {recipient}. Тон: {tone}.", "productivity", "ru"),
    // Learning - RU
    ("builtin-eli5-ru", "Объясни простыми словами", "Получить простое объяснение концепции", "Объясни {concept} простыми словами, понятными новичку. Используй аналогии и примеры.", "learning", "ru"),
    ("builtin-study-plan-ru", "План обучения", "Создать структурированный план обучения", "Создай структурированный план изучения {subject} на {duration}. Включи ресурсы, этапы и практические упражнения.", "learning", "ru"),
    // General - RU
    ("builtin-translate-ru", "Перевести", "Перевести текст на другой язык", "Переведи следующий текст на {language}:\n\n{text}", "general", "ru"),
    ("builtin-improve-prompt-ru", "Улучшить промпт", "Улучшить промпт для лучших ответов ИИ", "Улучши этот промпт, чтобы получать более качественные и детальные ответы от ИИ-ассистента:\n\n{prompt}", "general", "ru"),
];

#[tauri::command]
pub async fn seed_builtin_prompts(pool: State<'_, Pool>) -> Result<(), String> {
    for (id, title, desc, content, category, lang) in BUILTIN_PROMPTS_DATA {
        let now = uni_common::now_unix_secs();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO prompt_library (id, title, description, content, category, is_builtin, language, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(id)
        .bind(title)
        .bind(desc)
        .bind(content)
        .bind(category)
        .bind(lang)
        .bind(now)
        .bind(now)
        .execute(pool.inner())
        .await;
    }
    Ok(())
}
