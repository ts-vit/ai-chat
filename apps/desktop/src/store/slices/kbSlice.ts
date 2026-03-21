import { invoke } from "@tauri-apps/api/core";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import type { KnowledgeBase, KbDocument, KbSearchResultItem } from "../../types";
import type { ChatState } from "../chatStore";

export interface KbSlice {
    knowledgeBases: KnowledgeBase[];
    activeKbId: string | null;
    kbDocuments: KbDocument[];
    chatKbId: string | null;

    loadKnowledgeBases: () => Promise<void>;
    createKnowledgeBase: (name: string, description: string, embeddingModel?: string) => Promise<KnowledgeBase | null>;
    updateKnowledgeBase: (id: string, updates: Partial<KnowledgeBase>) => Promise<void>;
    deleteKnowledgeBase: (id: string) => Promise<void>;
    setActiveKbId: (id: string | null) => void;

    loadKbDocuments: (kbId: string) => Promise<void>;
    addKbDocuments: (kbId: string, filePaths: string[]) => Promise<void>;
    removeKbDocument: (kbId: string, documentId: string) => Promise<void>;

    indexKbDocument: (kbId: string, documentId: string) => Promise<void>;
    indexAllKbDocuments: (kbId: string) => Promise<void>;
    reindexKnowledgeBase: (kbId: string) => Promise<void>;

    attachKbToChat: (chatId: string, kbId: string) => Promise<void>;
    detachKbFromChat: (chatId: string) => Promise<void>;
    loadChatKb: (chatId: string) => Promise<void>;
    searchKnowledgeBase: (kbId: string, query: string, topK?: number) => Promise<KbSearchResultItem[]>;
    exportKnowledgeBase: (kbId: string) => Promise<void>;
    importKnowledgeBase: (zipPath: string) => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createKbSlice = (set: Set, _get: Get): KbSlice => ({
    knowledgeBases: [],
    activeKbId: null,
    kbDocuments: [],
    chatKbId: null,

    loadKnowledgeBases: async () => {
        try {
            const kbs = await invoke<KnowledgeBase[]>("list_knowledge_bases");
            set({ knowledgeBases: kbs });
        } catch (e) {
            notify.error(String(e));
        }
    },

    createKnowledgeBase: async (name: string, description: string, embeddingModel?: string) => {
        try {
            const kb = await invoke<KnowledgeBase>("create_knowledge_base", { name, description, embeddingModel });
            set((s) => ({ knowledgeBases: [kb, ...s.knowledgeBases], activeKbId: kb.id }));
            notify.success(i18n.t("kb.created"));
            return kb;
        } catch (e) {
            notify.error(String(e));
            return null;
        }
    },

    updateKnowledgeBase: async (id: string, updates: Partial<KnowledgeBase>) => {
        try {
            const state = _get();
            const existing = state.knowledgeBases.find((kb) => kb.id === id);
            if (!existing) return;

            await invoke("update_knowledge_base", {
                id,
                name: updates.name ?? existing.name,
                description: updates.description ?? existing.description,
                embeddingModel: updates.embeddingModel,
                embeddingDimensions: updates.embeddingDimensions,
                chunkingStrategy: updates.chunkingStrategy ?? existing.chunkingStrategy,
                chunkSize: updates.chunkSize ?? existing.chunkSize,
                chunkOverlap: updates.chunkOverlap ?? existing.chunkOverlap,
                minChunkSize: updates.minChunkSize ?? existing.minChunkSize,
                retrievalTopK: updates.retrievalTopK ?? existing.retrievalTopK,
                retrievalMinScore: updates.retrievalMinScore ?? existing.retrievalMinScore,
                queryRewritingEnabled: updates.queryRewritingEnabled ?? existing.queryRewritingEnabled,
                queryDecompositionEnabled: updates.queryDecompositionEnabled ?? existing.queryDecompositionEnabled,
                queryMaxVariants: updates.queryMaxVariants ?? existing.queryMaxVariants,
                rerankerType: updates.rerankerType ?? existing.rerankerType,
                rerankerOverfetchFactor: updates.rerankerOverfetchFactor ?? existing.rerankerOverfetchFactor,
                contextTokenBudget: updates.contextTokenBudget ?? existing.contextTokenBudget,
                contextSentenceExtraction: updates.contextSentenceExtraction ?? existing.contextSentenceExtraction,
                contextRedundancyRemoval: updates.contextRedundancyRemoval ?? existing.contextRedundancyRemoval,
                systemPrompt: updates.systemPrompt ?? existing.systemPrompt,
            });

            set((s) => ({
                knowledgeBases: s.knowledgeBases.map((kb) =>
                    kb.id === id ? { ...kb, ...updates, updatedAt: Math.floor(Date.now() / 1000) } : kb
                ),
            }));
            notify.success(i18n.t("kb.updated"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteKnowledgeBase: async (id: string) => {
        try {
            await invoke("delete_knowledge_base", { id });
            set((s) => ({
                knowledgeBases: s.knowledgeBases.filter((kb) => kb.id !== id),
                activeKbId: s.activeKbId === id ? null : s.activeKbId,
                kbDocuments: s.activeKbId === id ? [] : s.kbDocuments,
            }));
            notify.success(i18n.t("kb.deleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveKbId: (id: string | null) => {
        set({ activeKbId: id, kbDocuments: [] });
    },

    loadKbDocuments: async (kbId: string) => {
        try {
            const docs = await invoke<KbDocument[]>("list_kb_documents", { kbId });
            set({ kbDocuments: docs });
        } catch (e) {
            notify.error(String(e));
        }
    },

    addKbDocuments: async (kbId: string, filePaths: string[]) => {
        try {
            const docs = await invoke<KbDocument[]>("add_kb_documents_bulk", { kbId, filePaths });
            set((s) => ({
                kbDocuments: [...docs, ...s.kbDocuments],
                knowledgeBases: s.knowledgeBases.map((kb) =>
                    kb.id === kbId
                        ? { ...kb, documentCount: kb.documentCount + docs.length }
                        : kb
                ),
            }));
            notify.success(i18n.t("kb.documentsAdded"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    removeKbDocument: async (kbId: string, documentId: string) => {
        try {
            await invoke("remove_kb_document", { kbId, documentId });
            set((s) => ({
                kbDocuments: s.kbDocuments.filter((d) => d.id !== documentId),
                knowledgeBases: s.knowledgeBases.map((kb) =>
                    kb.id === kbId
                        ? { ...kb, documentCount: Math.max(kb.documentCount - 1, 0) }
                        : kb
                ),
            }));
            notify.success(i18n.t("kb.documentRemoved"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    indexKbDocument: async (kbId: string, documentId: string) => {
        try {
            await invoke("index_kb_document", { kbId, documentId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    indexAllKbDocuments: async (kbId: string) => {
        try {
            await invoke("index_all_kb_documents", { kbId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    reindexKnowledgeBase: async (kbId: string) => {
        try {
            await invoke("reindex_knowledge_base", { kbId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    attachKbToChat: async (chatId: string, kbId: string) => {
        try {
            await invoke("attach_kb_to_chat", { chatId, kbId });
            set({ chatKbId: kbId });
        } catch (e) {
            notify.error(String(e));
        }
    },

    detachKbFromChat: async (chatId: string) => {
        try {
            await invoke("detach_kb_from_chat", { chatId });
            set({ chatKbId: null });
        } catch (e) {
            notify.error(String(e));
        }
    },

    loadChatKb: async (chatId: string) => {
        try {
            const kb = await invoke<KnowledgeBase | null>("get_chat_kb", { chatId });
            set({ chatKbId: kb?.id ?? null });
        } catch {
            set({ chatKbId: null });
        }
    },

    searchKnowledgeBase: async (kbId: string, query: string, topK?: number) => {
        try {
            return await invoke<KbSearchResultItem[]>("search_knowledge_base", { kbId, query, topK });
        } catch (e) {
            notify.error(String(e));
            return [];
        }
    },

    exportKnowledgeBase: async (kbId: string) => {
        try {
            const path = await invoke<string>("export_knowledge_base", { kbId });
            notify.success(i18n.t("kb.exportSuccess", { path }));
        } catch (e) {
            const msg = String(e);
            if (!msg.includes("cancelled")) {
                notify.error(msg);
            }
        }
    },

    importKnowledgeBase: async (zipPath: string) => {
        try {
            const kb = await invoke<KnowledgeBase>("import_knowledge_base", { zipPath });
            set((s) => ({
                knowledgeBases: [kb, ...s.knowledgeBases],
                activeKbId: kb.id,
            }));
            notify.success(i18n.t("kb.importSuccess", { name: kb.name }));
            // Trigger auto-indexing of imported documents
            try {
                await invoke("index_all_kb_documents", { kbId: kb.id });
            } catch {
                // indexing runs in background, ignore errors
            }
        } catch (e) {
            notify.error(String(e));
        }
    },
});
