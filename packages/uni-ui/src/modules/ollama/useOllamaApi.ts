import { useState, useCallback, useMemo, useRef } from "react";
import type { OllamaModel, OllamaStatus, PullProgress } from "./types";

function formatSize(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(0)} MB`;
  return `${bytes} B`;
}

export function useOllamaApi(baseUrl: string) {
  const [status, setStatus] = useState<OllamaStatus>("unknown");
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [pullProgress, setPullProgress] = useState<PullProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const apiBase = useMemo(() => {
    let base = baseUrl || "http://localhost:11434/v1";
    base = base.replace(/\/v1\/?$/, "").replace(/\/$/, "");
    return base || "http://localhost:11434";
  }, [baseUrl]);

  const checkStatus = useCallback(async () => {
    setStatus("checking");
    try {
      const res = await fetch(apiBase, { signal: AbortSignal.timeout(3000) });
      setStatus(res.ok ? "available" : "unavailable");
    } catch {
      setStatus("unavailable");
    }
  }, [apiBase]);

  const loadModels = useCallback(async () => {
    try {
      const res = await fetch(`${apiBase}/api/tags`);
      const json = await res.json();
      const list: OllamaModel[] = (json.models ?? []).map(
        (m: { name: string; size?: number }) => ({
          name: m.name,
          size: m.size ? formatSize(m.size) : "",
        })
      );
      setModels(list);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [apiBase]);

  const pullModel = useCallback(
    async (modelName: string) => {
      setPullProgress({ model: modelName, progress: 0, status: "" });
      const controller = new AbortController();
      abortRef.current = controller;
      try {
        const res = await fetch(`${apiBase}/api/pull`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: modelName, stream: true }),
          signal: controller.signal,
        });
        const reader = res.body!.getReader();
        const decoder = new TextDecoder();
        let buffer = "";
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          while (buffer.includes("\n")) {
            const lineEnd = buffer.indexOf("\n");
            const line = buffer.slice(0, lineEnd).trim();
            buffer = buffer.slice(lineEnd + 1);
            if (!line) continue;
            try {
              const parsed = JSON.parse(line);
              if (parsed.status === "success") {
                setPullProgress(null);
                await loadModels();
                return;
              }
              if (parsed.completed && parsed.total && parsed.total > 0) {
                setPullProgress({
                  model: modelName,
                  progress: Math.round(
                    (parsed.completed / parsed.total) * 100
                  ),
                  status: parsed.status || "",
                });
              }
            } catch {
              /* ignore parse errors */
            }
          }
        }
        setPullProgress(null);
        await loadModels();
      } catch (e) {
        setPullProgress(null);
        if ((e as Error).name !== "AbortError") {
          throw e;
        }
      } finally {
        abortRef.current = null;
      }
    },
    [apiBase, loadModels]
  );

  const cancelPull = useCallback(() => {
    if (abortRef.current) {
      abortRef.current.abort();
    }
    setPullProgress(null);
  }, []);

  const deleteModel = useCallback(
    async (modelName: string) => {
      await fetch(`${apiBase}/api/delete`, {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: modelName }),
      });
      await loadModels();
    },
    [apiBase, loadModels]
  );

  return {
    status,
    models,
    pullProgress,
    error,
    checkStatus,
    loadModels,
    pullModel,
    deleteModel,
    cancelPull,
  };
}
