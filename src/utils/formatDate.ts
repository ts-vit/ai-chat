/** Порог: больше — значение в миллисекундах, иначе в секундах (Unix). */
const MS_THRESHOLD = 10_000_000_000;

/**
 * Приводит timestamp из БД к Unix-секундам (для отображения и передачи в API).
 * Старые записи могли быть в миллисекундах — не мигрируем, нормализуем на фронте.
 */
export function toUnixSeconds(timestamp: number): number {
    return timestamp > MS_THRESHOLD ? Math.floor(timestamp / 1000) : timestamp;
}

/**
 * Форматирует Unix timestamp (в секундах или мс — нормализуется внутри) в относительную дату для отображения в списке чатов.
 */
export function formatRelativeDate(timestamp: number): string {
    const sec = toUnixSeconds(timestamp);
    const date = new Date(sec * 1000);
    const now = new Date();

    const dateDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const nowDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const diffDays = Math.floor((nowDay - dateDay) / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "Сегодня";
    if (diffDays === 1) return "Вчера";

    return date.toLocaleDateString("ru-RU", { day: "numeric", month: "short" });
}
