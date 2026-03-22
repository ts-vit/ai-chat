export interface OllamaModel {
  name: string;
  size: string;
}

export type OllamaStatus = "unknown" | "checking" | "available" | "unavailable";

export interface PullProgress {
  model: string;
  progress: number;
  status: string;
}
