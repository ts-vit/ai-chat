import { invoke } from "@tauri-apps/api/core";

let lastText = "";
let lastCount = 0;

export async function countTokens(text: string): Promise<number> {
    if (!text.trim()) return 0;
    if (text === lastText) return lastCount;
    try {
        const count = await invoke<number>("count_tokens", { text });
        lastText = text;
        lastCount = count;
        return count;
    } catch {
        return Math.ceil(text.length / 4);
    }
}

export function formatTokenCount(count: number): string {
    if (count < 1000) return String(count);
    if (count < 10000) return (count / 1000).toFixed(1) + "k";
    return Math.round(count / 1000) + "k";
}
