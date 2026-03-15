import { invoke } from "@tauri-apps/api/core";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import type {
    ChatTemplate,
    Preset,
    Category,
    Snippet,
    PromptLibraryItem,
} from "../../types";
import type { ChatState } from "../chatStore";

export interface SnippetsSlice {
    // State
    snippets: Snippet[];
    welcomeSnippets: Snippet[];
    insertSnippetText: string | null;
    presets: Preset[];
    categories: Category[];
    templates: ChatTemplate[];
    promptLibrary: PromptLibraryItem[];
    promptLibraryLoading: boolean;

    // Actions — Presets
    loadPresets: () => Promise<void>;
    createPreset: (name: string, content: string, isDefault: boolean) => Promise<void>;
    updatePreset: (id: string, name: string, content: string, isDefault: boolean) => Promise<void>;
    deletePreset: (id: string) => Promise<void>;

    // Actions — Categories
    loadCategories: () => Promise<void>;
    createCategory: (name: string) => Promise<void>;
    updateCategory: (id: string, name: string) => Promise<void>;
    deleteCategory: (id: string) => Promise<void>;

    // Actions — Snippets
    loadSnippets: () => Promise<void>;
    loadWelcomeSnippets: () => Promise<void>;
    createSnippet: (name: string, content: string, categoryId: string, showOnWelcome?: boolean) => Promise<void>;
    updateSnippet: (id: string, name: string, content: string, categoryId: string, showOnWelcome?: boolean) => Promise<void>;
    deleteSnippet: (id: string) => Promise<void>;
    setInsertSnippetText: (text: string | null) => void;

    // Actions — Prompt Library
    fetchPromptLibrary: (category?: string, search?: string) => Promise<void>;
    createPromptLibraryItem: (data: { title: string; description: string; content: string; category: string }) => Promise<void>;
    updatePromptLibraryItem: (id: string, data: { title: string; description: string; content: string; category: string }) => Promise<void>;
    deletePromptLibraryItem: (id: string) => Promise<void>;

    // Actions — Templates
    loadTemplates: () => Promise<void>;
    createTemplate: (params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">) => Promise<void>;
    updateTemplate: (id: string, params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">) => Promise<void>;
    deleteTemplate: (id: string) => Promise<void>;
    reorderTemplates: (ids: string[]) => Promise<void>;
    createChatFromTemplate: (template: ChatTemplate) => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createSnippetsSlice = (set: Set, get: Get): SnippetsSlice => ({
    // ── State ──────────────────────────────────────────────

    snippets: [],
    welcomeSnippets: [],
    insertSnippetText: null,
    presets: [],
    categories: [],
    templates: [],
    promptLibrary: [],
    promptLibraryLoading: false,

    // ── Presets ────────────────────────────────────────────

    loadPresets: async () => {
        try {
            const list = await invoke<Preset[]>("get_all_presets");
            set({ presets: list ?? [] });
        } catch (e) {
            console.error("Failed to load presets:", e);
        }
    },

    createPreset: async (name, content, isDefault) => {
        try {
            const preset = await invoke<Preset>("create_preset", {
                name,
                content,
                isDefault,
            });
            set((state) => ({ presets: [...state.presets, preset] }));
            notify.success(i18n.t("notifications.presetCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updatePreset: async (id, name, content, isDefault) => {
        try {
            await invoke("update_preset", { id, name, content, isDefault });
            set((state) => ({
                presets: state.presets.map((p) =>
                    p.id === id ? { ...p, name, content, isDefault } : p
                ),
            }));
            notify.success(i18n.t("notifications.presetUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deletePreset: async (id) => {
        try {
            await invoke("delete_preset", { id });
            set((state) => ({ presets: state.presets.filter((p) => p.id !== id) }));
            notify.success(i18n.t("notifications.presetDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Categories ────────────────────────────────────────

    loadCategories: async () => {
        try {
            const list = await invoke<Category[]>("get_all_categories");
            set({ categories: list ?? [] });
        } catch (e) {
            console.error("Failed to load categories:", e);
        }
    },

    createCategory: async (name) => {
        try {
            const category = await invoke<Category>("create_category", { name });
            set((state) => ({ categories: [...state.categories, category] }));
            notify.success(i18n.t("notifications.categoryCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateCategory: async (id, name) => {
        try {
            await invoke("update_category", { id, name });
            set((state) => ({
                categories: state.categories.map((c) =>
                    c.id === id ? { ...c, name } : c
                ),
            }));
            notify.success(i18n.t("notifications.categoryUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteCategory: async (id) => {
        try {
            await invoke("delete_category", { id });
            set((state) => ({ categories: state.categories.filter((c) => c.id !== id) }));
            notify.success(i18n.t("notifications.categoryDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Snippets ──────────────────────────────────────────

    loadSnippets: async () => {
        try {
            const list = await invoke<Snippet[]>("get_all_snippets");
            set({ snippets: list ?? [] });
        } catch (e) {
            console.error("Failed to load snippets:", e);
        }
    },

    loadWelcomeSnippets: async () => {
        try {
            const list = await invoke<Snippet[]>("get_welcome_snippets");
            set({ welcomeSnippets: list ?? [] });
        } catch (e) {
            console.error("Failed to load welcome snippets:", e);
        }
    },

    createSnippet: async (name, content, categoryId, showOnWelcome) => {
        try {
            const snippet = await invoke<Snippet>("create_snippet", {
                name,
                content,
                categoryId,
                showOnWelcome,
            });
            set((state) => ({ snippets: [...state.snippets, snippet] }));
            notify.success(i18n.t("notifications.snippetCreated"));
            get().loadWelcomeSnippets();
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateSnippet: async (id, name, content, categoryId, showOnWelcome) => {
        try {
            await invoke("update_snippet", { id, name, content, categoryId, showOnWelcome });
            set((state) => ({
                snippets: state.snippets.map((s) =>
                    s.id === id ? { ...s, name, content, categoryId, showOnWelcome } : s
                ),
            }));
            notify.success(i18n.t("notifications.snippetUpdated"));
            get().loadWelcomeSnippets();
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteSnippet: async (id) => {
        try {
            await invoke("delete_snippet", { id });
            set((state) => ({ snippets: state.snippets.filter((s) => s.id !== id) }));
            notify.success(i18n.t("notifications.snippetDeleted"));
            get().loadWelcomeSnippets();
        } catch (e) {
            notify.error(String(e));
        }
    },

    setInsertSnippetText: (text) => set({ insertSnippetText: text }),

    // ── Prompt Library ────────────────────────────────────

    fetchPromptLibrary: async (category, search) => {
        set({ promptLibraryLoading: true });
        try {
            const lang = get().settings.language || "en";
            const items = await invoke<PromptLibraryItem[]>("get_prompt_library", {
                category: category || null,
                search: search || null,
                language: lang,
            });
            set({ promptLibrary: items ?? [] });
        } catch (e) {
            notify.error(String(e));
        } finally {
            set({ promptLibraryLoading: false });
        }
    },

    createPromptLibraryItem: async (data) => {
        try {
            const lang = get().settings.language || "en";
            await invoke("create_prompt_library_item", { ...data, language: lang });
            await get().fetchPromptLibrary();
        } catch (e) {
            notify.error(String(e));
        }
    },

    updatePromptLibraryItem: async (id, data) => {
        try {
            await invoke("update_prompt_library_item", { id, ...data });
            await get().fetchPromptLibrary();
        } catch (e) {
            notify.error(String(e));
        }
    },

    deletePromptLibraryItem: async (id) => {
        try {
            await invoke("delete_prompt_library_item", { id });
            set((state) => ({ promptLibrary: state.promptLibrary.filter((p) => p.id !== id) }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    // ── Templates ─────────────────────────────────────────

    loadTemplates: async () => {
        try {
            const list = await invoke<ChatTemplate[]>("get_all_templates");
            set({ templates: list ?? [] });
        } catch (e) {
            console.error("Failed to load templates:", e);
        }
    },

    createTemplate: async (
        params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">
    ) => {
        try {
            const template = await invoke<ChatTemplate>("create_template", {
                name: params.name,
                icon: params.icon ?? null,
                providerId: params.providerId,
                model: params.model,
                systemPrompt: params.systemPrompt ?? "",
                temperature: params.temperature ?? null,
                maxTokens: params.maxTokens ?? null,
                topP: params.topP ?? null,
                topK: params.topK ?? null,
                frequencyPenalty: params.frequencyPenalty ?? null,
                presencePenalty: params.presencePenalty ?? null,
            });
            set((state) => ({ templates: [...state.templates, template] }));
            notify.success(i18n.t("notifications.templateCreated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    updateTemplate: async (
        id: string,
        params: Omit<ChatTemplate, "id" | "sortOrder" | "createdAt">
    ) => {
        try {
            await invoke("update_template", {
                id,
                name: params.name,
                icon: params.icon ?? null,
                providerId: params.providerId,
                model: params.model,
                systemPrompt: params.systemPrompt ?? "",
                temperature: params.temperature ?? null,
                maxTokens: params.maxTokens ?? null,
                topP: params.topP ?? null,
                topK: params.topK ?? null,
                frequencyPenalty: params.frequencyPenalty ?? null,
                presencePenalty: params.presencePenalty ?? null,
            });
            set((state) => ({
                templates: state.templates.map((t) =>
                    t.id === id
                        ? {
                              ...t,
                              name: params.name,
                              icon: params.icon ?? t.icon,
                              providerId: params.providerId,
                              model: params.model,
                              systemPrompt: params.systemPrompt ?? "",
                              temperature: params.temperature ?? undefined,
                              maxTokens: params.maxTokens ?? undefined,
                              topP: params.topP ?? undefined,
                              topK: params.topK ?? undefined,
                              frequencyPenalty: params.frequencyPenalty ?? undefined,
                              presencePenalty: params.presencePenalty ?? undefined,
                          }
                        : t
                ),
            }));
            notify.success(i18n.t("notifications.templateUpdated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteTemplate: async (id: string) => {
        try {
            await invoke("delete_template", { id });
            set((state) => ({
                templates: state.templates.filter((t) => t.id !== id),
            }));
            notify.success(i18n.t("notifications.templateDeleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    reorderTemplates: async (ids: string[]) => {
        try {
            await invoke("reorder_templates", { templateIds: ids });
            set((state) => {
                const byId = new Map(state.templates.map((t) => [t.id, t]));
                const reordered = ids
                    .map((id, index) => {
                        const t = byId.get(id);
                        return t ? { ...t, sortOrder: index } : null;
                    })
                    .filter((t): t is ChatTemplate => t !== null);
                const remaining = state.templates.filter((t) => !byId.has(t.id));
                return { templates: [...reordered, ...remaining] };
            });
        } catch (e) {
            notify.error(String(e));
        }
    },

    createChatFromTemplate: async (template: ChatTemplate) => {
        try {
            const isImageModel =
                get().models.find((m) => m.id === template.model)
                    ?.supportsImageGeneration ?? false;
            await get().createChat(
                template.providerId,
                template.model,
                isImageModel,
                {
                    temperature: template.temperature ?? undefined,
                    maxTokens: template.maxTokens ?? undefined,
                    topP: template.topP ?? undefined,
                    topK: template.topK ?? undefined,
                    frequencyPenalty: template.frequencyPenalty ?? undefined,
                    presencePenalty: template.presencePenalty ?? undefined,
                }
            );
            const { activeChatId } = get();
            if (activeChatId && template.systemPrompt?.trim()) {
                await get().setChatSystemPrompt(activeChatId, template.systemPrompt);
            }
        } catch (e) {
            notify.error(String(e));
        }
    },
});
