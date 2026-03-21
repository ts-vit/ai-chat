export const PROMPT_CATEGORIES = [
    { id: 'coding', icon: 'IconCode', label: { en: 'Coding', ru: 'Код' } },
    { id: 'writing', icon: 'IconPencil', label: { en: 'Writing', ru: 'Тексты' } },
    { id: 'analysis', icon: 'IconChartBar', label: { en: 'Analysis', ru: 'Анализ' } },
    { id: 'creative', icon: 'IconBrush', label: { en: 'Creative', ru: 'Креатив' } },
    { id: 'productivity', icon: 'IconRocket', label: { en: 'Productivity', ru: 'Продуктивность' } },
    { id: 'learning', icon: 'IconSchool', label: { en: 'Learning', ru: 'Обучение' } },
    { id: 'general', icon: 'IconMessage', label: { en: 'General', ru: 'Общее' } },
] as const;

export type PromptCategoryId = (typeof PROMPT_CATEGORIES)[number]['id'];
