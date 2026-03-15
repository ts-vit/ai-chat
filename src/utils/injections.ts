import { invoke } from "@tauri-apps/api/core";
import { readText } from "@tauri-apps/plugin-clipboard-manager";

/**
 * Resolve all {{variable}} placeholders in text.
 *
 * Supported:
 * - {{date}} — current date/time as "YYYY-MM-DD HH:mm"
 * - {{date:FORMAT}} — custom format (YYYY, MM, DD, HH, mm, ss, ddd, dddd)
 * - {{clipboard}} — clipboard text
 * - {{file:path}} — file contents from disk
 *
 * Unknown placeholders are left as-is.
 * Failed placeholders are replaced with [Error: message].
 */
export async function resolveInjections(text: string): Promise<string> {
  if (!text.includes("{{")) return text;

  const pattern = /\{\{([^}]+)\}\}/g;
  const matches: Array<{ full: string; inner: string }> = [];
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    matches.push({ full: match[0], inner: match[1].trim() });
  }

  if (matches.length === 0) return text;

  const replacements = new Map<string, string>();

  for (const m of matches) {
    if (replacements.has(m.full)) continue;

    try {
      const resolved = await resolveSingle(m.inner);
      if (resolved !== null) {
        replacements.set(m.full, resolved);
      }
    } catch (err) {
      replacements.set(m.full, `[Error: ${err}]`);
    }
  }

  let result = text;
  for (const [placeholder, value] of replacements) {
    result = result.split(placeholder).join(value);
  }

  return result;
}

async function resolveSingle(inner: string): Promise<string | null> {
  if (inner === "date") {
    return formatDate("YYYY-MM-DD HH:mm");
  }
  if (inner.startsWith("date:")) {
    const fmt = inner.slice(5).trim();
    return formatDate(fmt || "YYYY-MM-DD HH:mm");
  }

  if (inner === "clipboard") {
    try {
      const clipText = await readText();
      return clipText || "";
    } catch {
      return "[clipboard unavailable]";
    }
  }

  if (inner.startsWith("file:")) {
    const filePath = inner.slice(5).trim();
    if (!filePath) return "[Error: empty file path]";
    const contents: string = await invoke("read_file_contents", { path: filePath });
    return contents;
  }

  return null;
}

function formatDate(fmt: string): string {
  const now = new Date();
  const pad = (n: number) => n.toString().padStart(2, "0");

  const days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  const daysFull = [
    "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
  ];

  return fmt
    .replace("YYYY", now.getFullYear().toString())
    .replace("MM", pad(now.getMonth() + 1))
    .replace("DD", pad(now.getDate()))
    .replace("HH", pad(now.getHours()))
    .replace("mm", pad(now.getMinutes()))
    .replace("ss", pad(now.getSeconds()))
    .replace("dddd", daysFull[now.getDay()])
    .replace("ddd", days[now.getDay()]);
}

/**
 * Check if text contains any {{...}} patterns.
 */
export function hasInjections(text: string): boolean {
  return /\{\{[^}]+\}\}/.test(text);
}
