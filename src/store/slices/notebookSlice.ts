import { invoke } from "@tauri-apps/api/core";
import i18n from "../../i18n";
import { notify } from "../../utils/notify";
import type { Notebook, KbDocument } from "../../types";
import type { ChatState } from "../chatStore";

export interface NotebookSlice {
    notebooks: Notebook[];
    activeNotebookId: string | null;
    notebookDocuments: KbDocument[];
    notebookLoading: boolean;

    loadNotebooks: () => Promise<void>;
    createNotebook: (name: string, description: string) => Promise<Notebook | null>;
    updateNotebook: (id: string, name: string, description: string) => Promise<void>;
    deleteNotebook: (id: string) => Promise<void>;
    setActiveNotebook: (id: string | null) => void;
    loadNotebookDocuments: (notebookId: string) => Promise<void>;
    addNotebookDocuments: (notebookId: string, filePaths: string[]) => Promise<void>;
    addNotebookSource: (
        notebookId: string,
        sourceType: string,
        opts?: { filePaths?: string[]; url?: string; name?: string; content?: string }
    ) => Promise<void>;
    removeNotebookDocument: (notebookId: string, documentId: string) => Promise<void>;
}

type Set = (partial: Partial<ChatState> | ((state: ChatState) => Partial<ChatState>)) => void;
type Get = () => ChatState;

export const createNotebookSlice = (set: Set, _get: Get): NotebookSlice => ({
    notebooks: [],
    activeNotebookId: null,
    notebookDocuments: [],
    notebookLoading: false,

    loadNotebooks: async () => {
        try {
            const notebooks = await invoke<Notebook[]>("list_notebooks");
            set({ notebooks });
        } catch (e) {
            notify.error(String(e));
        }
    },

    createNotebook: async (name: string, description: string) => {
        try {
            const notebook = await invoke<Notebook>("create_notebook", { name, description });
            set((s) => ({ notebooks: [notebook, ...s.notebooks] }));
            notify.success(i18n.t("notebook.created"));
            return notebook;
        } catch (e) {
            notify.error(String(e));
            return null;
        }
    },

    updateNotebook: async (id: string, name: string, description: string) => {
        try {
            await invoke("update_notebook", { id, name, description });
            set((s) => ({
                notebooks: s.notebooks.map((n) =>
                    n.id === id ? { ...n, name, description, updatedAt: Math.floor(Date.now() / 1000) } : n
                ),
            }));
        } catch (e) {
            notify.error(String(e));
        }
    },

    deleteNotebook: async (id: string) => {
        try {
            await invoke("delete_notebook", { id });
            set((s) => ({
                notebooks: s.notebooks.filter((n) => n.id !== id),
                activeNotebookId: s.activeNotebookId === id ? null : s.activeNotebookId,
                notebookDocuments: s.activeNotebookId === id ? [] : s.notebookDocuments,
            }));
            notify.success(i18n.t("notebook.deleted"));
        } catch (e) {
            notify.error(String(e));
        }
    },

    setActiveNotebook: (id: string | null) => {
        set({ activeNotebookId: id, notebookDocuments: [] });
    },

    loadNotebookDocuments: async (notebookId: string) => {
        try {
            const docs = await invoke<KbDocument[]>("list_notebook_documents", { notebookId });
            set({ notebookDocuments: docs });
        } catch (e) {
            notify.error(String(e));
        }
    },

    addNotebookDocuments: async (notebookId: string, filePaths: string[]) => {
        try {
            await invoke("add_notebook_documents", { notebookId, filePaths });
            // Reload documents
            const docs = await invoke<KbDocument[]>("list_notebook_documents", { notebookId });
            set({ notebookDocuments: docs });
            // Reload notebooks to update stats
            const notebooks = await invoke<Notebook[]>("list_notebooks");
            set({ notebooks });
        } catch (e) {
            notify.error(String(e));
        }
    },

    addNotebookSource: async (notebookId, sourceType, opts) => {
        try {
            set({ notebookLoading: true });
            await invoke("add_notebook_source", {
                notebookId,
                sourceType,
                filePaths: opts?.filePaths ?? null,
                url: opts?.url ?? null,
                name: opts?.name ?? null,
                content: opts?.content ?? null,
            });
            // Reload documents
            const docs = await invoke<KbDocument[]>("list_notebook_documents", { notebookId });
            set({ notebookDocuments: docs, notebookLoading: false });
            // Reload notebooks to update stats
            const notebooks = await invoke<Notebook[]>("list_notebooks");
            set({ notebooks });
        } catch (e) {
            notify.error(String(e));
            set({ notebookLoading: false });
        }
    },

    removeNotebookDocument: async (notebookId: string, documentId: string) => {
        try {
            await invoke("remove_notebook_document", { notebookId, documentId });
            set((s) => ({
                notebookDocuments: s.notebookDocuments.filter((d) => d.id !== documentId),
            }));
            // Reload notebooks to update stats
            const notebooks = await invoke<Notebook[]>("list_notebooks");
            set({ notebooks });
        } catch (e) {
            notify.error(String(e));
        }
    },
});
