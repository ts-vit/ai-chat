/**
 * Форматирует Unix timestamp (в секундах) в относительную дату для отображения в списке чатов.
 */
export function formatRelativeDate(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();

    const dateDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    const nowDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const diffDays = Math.floor((nowDay - dateDay) / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "Сегодня";
    if (diffDays === 1) return "Вчера";

    return date.toLocaleDateString("ru-RU", { day: "numeric", month: "short" });
}
