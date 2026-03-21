export const CRON_PRESETS = [
    { labelEn: "Every hour", labelRu: "Каждый час", cron: "0 * * * *" },
    { labelEn: "Every day at 9:00", labelRu: "Каждый день в 9:00", cron: "0 9 * * *" },
    { labelEn: "Every day at 20:00", labelRu: "Каждый день в 20:00", cron: "0 20 * * *" },
    { labelEn: "Every Monday at 10:00", labelRu: "Каждый понедельник в 10:00", cron: "0 10 * * 1" },
    { labelEn: "Every Friday at 18:00", labelRu: "Каждую пятницу в 18:00", cron: "0 18 * * 5" },
    { labelEn: "Every 30 minutes", labelRu: "Каждые 30 минут", cron: "*/30 * * * *" },
    { labelEn: "Every 6 hours", labelRu: "Каждые 6 часов", cron: "0 */6 * * *" },
];

const DAYS_EN = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const DAYS_RU = ["воскресенье", "понедельник", "вторник", "среду", "четверг", "пятницу", "субботу"];

export function cronToHuman(cron: string, locale: string): string {
    const parts = cron.trim().split(/\s+/);
    if (parts.length !== 5) return cron;

    const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;
    const isRu = locale.startsWith("ru");

    // Every N minutes: */N * * * *
    if (minute.startsWith("*/") && hour === "*" && dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
        const n = minute.slice(2);
        return isRu ? `Каждые ${n} минут` : `Every ${n} minutes`;
    }

    // Every hour: 0 * * * *
    if (minute === "0" && hour === "*" && dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
        return isRu ? "Каждый час" : "Every hour";
    }

    // Every N hours: 0 */N * * *
    if (minute === "0" && hour.startsWith("*/") && dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
        const n = hour.slice(2);
        return isRu ? `Каждые ${n} часов` : `Every ${n} hours`;
    }

    // Every day at HH:MM: M H * * *
    if (/^\d+$/.test(minute) && /^\d+$/.test(hour) && dayOfMonth === "*" && month === "*" && dayOfWeek === "*") {
        const time = `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`;
        return isRu ? `Каждый день в ${time}` : `Every day at ${time}`;
    }

    // Specific day of week: M H * * D
    if (/^\d+$/.test(minute) && /^\d+$/.test(hour) && dayOfMonth === "*" && month === "*" && /^\d+$/.test(dayOfWeek)) {
        const time = `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`;
        const dayIdx = parseInt(dayOfWeek) % 7;
        const dayName = isRu ? DAYS_RU[dayIdx] : DAYS_EN[dayIdx];
        return isRu ? `Каждый(ую) ${dayName} в ${time}` : `Every ${dayName} at ${time}`;
    }

    return cron;
}
