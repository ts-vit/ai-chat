import { useState, useRef, useCallback, useEffect } from "react";
import type { OpenRouterModel } from "./types";

interface UseOpenRouterModelsResult {
  models: OpenRouterModel[];
  loading: boolean;
  error: string | null;
  reload: (force?: boolean) => void;
}

export function useOpenRouterModels(): UseOpenRouterModelsResult {
  const [models, setModels] = useState<OpenRouterModel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cacheRef = useRef<OpenRouterModel[] | null>(null);

  const load = useCallback(async (force = false) => {
    if (!force && cacheRef.current) {
      setModels(cacheRef.current);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const res = await fetch("https://openrouter.ai/api/v1/models");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      const data: OpenRouterModel[] = (json.data ?? []).map((m: Record<string, unknown>) => ({
        id: m.id as string,
        name: (m.name as string) || (m.id as string),
        pricing: {
          prompt: String((m.pricing as Record<string, unknown>)?.prompt ?? "0"),
          completion: String((m.pricing as Record<string, unknown>)?.completion ?? "0"),
        },
        context_length: (m.context_length as number) ?? 0,
        description: (m.description as string) || undefined,
      }));
      cacheRef.current = data;
      setModels(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return { models, loading, error, reload: load };
}
