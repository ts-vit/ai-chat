import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Card,
    Collapse,
    Group,
    Loader,
    NumberInput,
    ScrollArea,
    Select,
    Slider,
    Stack,
    Switch,
    Text,
    Textarea,
    TextInput,
    Title,
    Tooltip,
} from "@mantine/core";
import {
    IconChevronDown,
    IconChevronRight,
    IconDatabase,
    IconDownload,
    IconFile,
    IconPlus,
    IconRefresh,
    IconTrash,
    IconUpload,
    IconX,
} from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useChatStore } from "../store/chatStore";
import { ConfirmModal } from "./ConfirmModal";
import { CreateKnowledgeBaseModal } from "./CreateKnowledgeBaseModal";
import type { KnowledgeBase, KbSearchResultItem } from "../types";

interface KbIndexingProgress {
    kbId: string;
    documentId: string;
    status: "parsing" | "embedding" | "storing" | "done" | "failed";
    totalChunks?: number;
    error?: string;
}

function formatFileSize(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

function indexingStatusColor(status: string): string {
    switch (status) {
        case "indexed": return "green";
        case "pending": return "gray";
        case "indexing": return "blue";
        case "failed": return "red";
        default: return "gray";
    }
}

export function KnowledgeBasesPage() {
    const { t } = useTranslation();
    const {
        knowledgeBases,
        activeKbId,
        kbDocuments,
        loadKnowledgeBases,
        updateKnowledgeBase,
        deleteKnowledgeBase,
        setActiveKbId,
        loadKbDocuments,
        addKbDocuments,
        removeKbDocument,
        indexKbDocument,
        indexAllKbDocuments,
        reindexKnowledgeBase,
        searchKnowledgeBase,
        exportKnowledgeBase,
        importKnowledgeBase,
    } = useChatStore();

    const [createModalOpen, setCreateModalOpen] = useState(false);
    const [deleteKbId, setDeleteKbId] = useState<string | null>(null);
    const [deleteDocId, setDeleteDocId] = useState<string | null>(null);
    const [reindexKbId, setReindexKbId] = useState<string | null>(null);
    const [settingsOpen, setSettingsOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState("");
    const [searchResults, setSearchResults] = useState<KbSearchResultItem[]>([]);
    const [searchLoading, setSearchLoading] = useState(false);
    const [searchPerformed, setSearchPerformed] = useState(false);

    // Editable fields for the active KB
    const activeKb = knowledgeBases.find((kb) => kb.id === activeKbId) ?? null;
    const [editName, setEditName] = useState("");
    const [editDescription, setEditDescription] = useState("");
    const [editChunkingStrategy, setEditChunkingStrategy] = useState("tokens");
    const [editChunkSize, setEditChunkSize] = useState(512);
    const [editChunkOverlap, setEditChunkOverlap] = useState(50);
    const [editMinChunkSize, setEditMinChunkSize] = useState(50);
    const [editTopK, setEditTopK] = useState(5);
    const [editMinScore, setEditMinScore] = useState(0.7);
    const [editSystemPrompt, setEditSystemPrompt] = useState("");
    const [editEmbeddingModel, setEditEmbeddingModel] = useState("e5-small");
    const [editQueryRewriting, setEditQueryRewriting] = useState(true);
    const [editQueryDecomposition, setEditQueryDecomposition] = useState(false);
    const [editQueryMaxVariants, setEditQueryMaxVariants] = useState(3);
    const [editRerankerType, setEditRerankerType] = useState("none");
    const [editRerankerOverfetchFactor, setEditRerankerOverfetchFactor] = useState(4);
    const [editContextTokenBudget, setEditContextTokenBudget] = useState(4000);
    const [editContextSentenceExtraction, setEditContextSentenceExtraction] = useState(true);
    const [editContextRedundancyRemoval, setEditContextRedundancyRemoval] = useState(true);
    const [dirty, setDirty] = useState(false);

    useEffect(() => {
        loadKnowledgeBases();
    }, [loadKnowledgeBases]);

    useEffect(() => {
        if (activeKbId) {
            loadKbDocuments(activeKbId);
        }
    }, [activeKbId, loadKbDocuments]);

    // Sync edit fields when active KB changes
    useEffect(() => {
        if (activeKb) {
            setEditName(activeKb.name);
            setEditDescription(activeKb.description);
            setEditChunkingStrategy(activeKb.chunkingStrategy);
            setEditChunkSize(activeKb.chunkSize);
            setEditChunkOverlap(activeKb.chunkOverlap);
            setEditMinChunkSize(activeKb.minChunkSize ?? 50);
            setEditTopK(activeKb.retrievalTopK);
            setEditMinScore(activeKb.retrievalMinScore);
            setEditQueryRewriting(activeKb.queryRewritingEnabled ?? true);
            setEditQueryDecomposition(activeKb.queryDecompositionEnabled ?? false);
            setEditQueryMaxVariants(activeKb.queryMaxVariants ?? 3);
            setEditRerankerType(activeKb.rerankerType ?? "none");
            setEditRerankerOverfetchFactor(activeKb.rerankerOverfetchFactor ?? 4);
            setEditContextTokenBudget(activeKb.contextTokenBudget ?? 4000);
            setEditContextSentenceExtraction(activeKb.contextSentenceExtraction ?? true);
            setEditContextRedundancyRemoval(activeKb.contextRedundancyRemoval ?? true);
            setEditSystemPrompt(activeKb.systemPrompt);
            setEditEmbeddingModel(activeKb.embeddingModel);
            setDirty(false);
        }
    }, [activeKb?.id]);

    // Listen for indexing progress events
    useEffect(() => {
        const unlistenPromise = listen<KbIndexingProgress>("kb-indexing-progress", (event) => {
            const { kbId, status } = event.payload;
            if (status === "done" || status === "failed") {
                // Reload documents and KB data
                if (kbId === activeKbId) {
                    loadKbDocuments(kbId);
                }
                loadKnowledgeBases();
            }
        });
        return () => { unlistenPromise.then(fn => fn()); };
    }, [activeKbId, loadKbDocuments, loadKnowledgeBases]);

    const embeddingDimsMap: Record<string, number> = { "e5-small": 384, "openai": 1536, "gemini": 768 };

    const handleSave = useCallback(async () => {
        if (!activeKbId || !activeKb) return;
        const embeddingChanged = editEmbeddingModel !== activeKb.embeddingModel;
        await updateKnowledgeBase(activeKbId, {
            name: editName,
            description: editDescription,
            embeddingModel: embeddingChanged ? editEmbeddingModel : undefined,
            embeddingDimensions: embeddingChanged ? embeddingDimsMap[editEmbeddingModel] ?? 384 : undefined,
            chunkingStrategy: editChunkingStrategy,
            chunkSize: editChunkSize,
            chunkOverlap: editChunkOverlap,
            minChunkSize: editMinChunkSize,
            retrievalTopK: editTopK,
            retrievalMinScore: editMinScore,
            queryRewritingEnabled: editQueryRewriting,
            queryDecompositionEnabled: editQueryDecomposition,
            queryMaxVariants: editQueryMaxVariants,
            rerankerType: editRerankerType,
            rerankerOverfetchFactor: editRerankerOverfetchFactor,
            contextTokenBudget: editContextTokenBudget,
            contextSentenceExtraction: editContextSentenceExtraction,
            contextRedundancyRemoval: editContextRedundancyRemoval,
            systemPrompt: editSystemPrompt,
        });
        setDirty(false);
    }, [activeKbId, activeKb, editName, editDescription, editEmbeddingModel, editChunkingStrategy, editChunkSize, editChunkOverlap, editMinChunkSize, editTopK, editMinScore, editQueryRewriting, editQueryDecomposition, editQueryMaxVariants, editRerankerType, editRerankerOverfetchFactor, editContextTokenBudget, editContextSentenceExtraction, editContextRedundancyRemoval, editSystemPrompt, updateKnowledgeBase]);

    const handleAddDocuments = useCallback(async () => {
        if (!activeKbId) return;
        const selected = await open({
            multiple: true,
            filters: [{ name: "Documents", extensions: ["txt", "md", "html", "pdf", "docx", "json", "csv", "xml", "rst", "rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "c", "cpp", "h", "rb", "php", "swift", "kt", "cs", "yaml", "yml", "toml"] }],
        });
        if (!selected) return;
        const paths = Array.isArray(selected) ? selected : [selected];
        if (paths.length > 0) {
            await addKbDocuments(activeKbId, paths);
        }
    }, [activeKbId, addKbDocuments]);

    const handleDeleteKb = useCallback(async () => {
        if (!deleteKbId) return;
        await deleteKnowledgeBase(deleteKbId);
        setDeleteKbId(null);
    }, [deleteKbId, deleteKnowledgeBase]);

    const handleDeleteDoc = useCallback(async () => {
        if (!deleteDocId || !activeKbId) return;
        await removeKbDocument(activeKbId, deleteDocId);
        setDeleteDocId(null);
    }, [deleteDocId, activeKbId, removeKbDocument]);

    const handleReindex = useCallback(async () => {
        if (!reindexKbId) return;
        await reindexKnowledgeBase(reindexKbId);
        setReindexKbId(null);
    }, [reindexKbId, reindexKnowledgeBase]);

    const handleImport = useCallback(async () => {
        const selected = await open({
            multiple: false,
            filters: [{ name: "UNI AI Knowledge Base", extensions: ["zip"] }],
        });
        if (!selected) return;
        const path = Array.isArray(selected) ? selected[0] : selected;
        if (path) {
            await importKnowledgeBase(path);
        }
    }, [importKnowledgeBase]);

    const handleSearch = useCallback(async () => {
        if (!activeKbId || !searchQuery.trim()) return;
        setSearchLoading(true);
        const results = await searchKnowledgeBase(activeKbId, searchQuery.trim());
        setSearchResults(results);
        setSearchPerformed(true);
        setSearchLoading(false);
    }, [activeKbId, searchQuery, searchKnowledgeBase]);

    // Reset search when switching KB
    useEffect(() => {
        setSearchQuery("");
        setSearchResults([]);
        setSearchPerformed(false);
    }, [activeKbId]);

    const deleteKbName = knowledgeBases.find((kb) => kb.id === deleteKbId)?.name ?? "";
    const deleteDocName = kbDocuments.find((d) => d.id === deleteDocId)?.name ?? "";
    const reindexKbName = knowledgeBases.find((kb) => kb.id === reindexKbId)?.name ?? "";

    const markDirty = useCallback(() => setDirty(true), []);

    const hasPendingDocs = useMemo(
        () => kbDocuments.some((d) => d.indexingStatus === "pending"),
        [kbDocuments]
    );
    const hasIndexingDocs = useMemo(
        () => kbDocuments.some((d) => d.indexingStatus === "indexing"),
        [kbDocuments]
    );

    return (
        <Box p="md" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
            {/* Header */}
            <Group justify="space-between" mb="md">
                <Group gap="sm">
                    <IconDatabase size={24} stroke={1.5} />
                    <Title order={3}>{t("kb.title")}</Title>
                    <Badge variant="light" size="lg">{knowledgeBases.length}</Badge>
                </Group>
                <Group gap="xs">
                    <Button variant="light" leftSection={<IconUpload size={16} stroke={1.5} />} size="sm" onClick={handleImport}>
                        {t("kb.import")}
                    </Button>
                    <Button leftSection={<IconPlus size={16} stroke={1.5} />} size="sm" onClick={() => setCreateModalOpen(true)}>
                        {t("kb.create")}
                    </Button>
                    <Tooltip label={t("common.close")}>
                        <ActionIcon variant="subtle" size="lg" onClick={() => useChatStore.getState().setView("chat")}>
                            <IconX size={20} stroke={1.5} />
                        </ActionIcon>
                    </Tooltip>
                </Group>
            </Group>

            {/* Two-panel layout */}
            <Box style={{ display: "flex", flex: 1, gap: 16, minHeight: 0, overflow: "hidden" }}>
                {/* Left panel — KB list */}
                <Box style={{ width: 300, minWidth: 260, flexShrink: 0, display: "flex", flexDirection: "column" }}>
                    <ScrollArea style={{ flex: 1 }} offsetScrollbars>
                        <Stack gap="xs">
                            {knowledgeBases.length === 0 ? (
                                <Box ta="center" py="xl">
                                    <IconDatabase size={48} stroke={1} style={{ opacity: 0.3 }} />
                                    <Text size="sm" c="dimmed" mt="sm">{t("kb.noKbs")}</Text>
                                    <Text size="xs" c="dimmed">{t("kb.noKbsHint")}</Text>
                                </Box>
                            ) : (
                                knowledgeBases.map((kb) => (
                                    <KbCard
                                        key={kb.id}
                                        kb={kb}
                                        active={kb.id === activeKbId}
                                        onClick={() => setActiveKbId(kb.id)}
                                        onDelete={() => setDeleteKbId(kb.id)}
                                        t={t}
                                    />
                                ))
                            )}
                        </Stack>
                    </ScrollArea>
                </Box>

                {/* Right panel — KB detail */}
                <Box style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
                    {activeKb ? (
                        <ScrollArea style={{ flex: 1 }} offsetScrollbars>
                            <Stack gap="md" pr="xs">
                                {/* Name & Description */}
                                <TextInput
                                    label={t("kb.name")}
                                    value={editName}
                                    onChange={(e) => { setEditName(e.currentTarget.value); markDirty(); }}
                                />
                                <Textarea
                                    label={t("kb.description")}
                                    value={editDescription}
                                    onChange={(e) => { setEditDescription(e.currentTarget.value); markDirty(); }}
                                    minRows={2}
                                    maxRows={4}
                                    autosize
                                />

                                {dirty && (
                                    <Group>
                                        <Button size="xs" onClick={handleSave}>{t("common.save")}</Button>
                                    </Group>
                                )}

                                {/* Documents */}
                                <Group justify="space-between">
                                    <Group gap="xs">
                                        <Text fw={600} size="sm">{t("kb.documents")} ({kbDocuments.length})</Text>
                                        {hasIndexingDocs && <Loader size={14} />}
                                    </Group>
                                    <Group gap="xs">
                                        {hasPendingDocs && (
                                            <Button
                                                size="xs"
                                                variant="light"
                                                leftSection={<IconRefresh size={14} stroke={1.5} />}
                                                onClick={() => activeKbId && indexAllKbDocuments(activeKbId)}
                                            >
                                                {t("kb.indexAll")}
                                            </Button>
                                        )}
                                        <Button size="xs" variant="light" leftSection={<IconPlus size={14} stroke={1.5} />} onClick={handleAddDocuments}>
                                            {t("kb.addDocuments")}
                                        </Button>
                                    </Group>
                                </Group>

                                {kbDocuments.length === 0 ? (
                                    <Box ta="center" py="md" style={{ border: "1px dashed var(--mantine-color-default-border)", borderRadius: 8 }}>
                                        <Text size="sm" c="dimmed">{t("kb.noDocuments")}</Text>
                                        <Text size="xs" c="dimmed">{t("kb.noDocumentsHint")}</Text>
                                    </Box>
                                ) : (
                                    <Stack gap={4}>
                                        {kbDocuments.map((doc) => (
                                            <Group key={doc.id} justify="space-between" px="xs" py={6}
                                                style={{ borderRadius: 6, backgroundColor: "var(--mantine-color-default-hover)" }}>
                                                <Group gap="xs" style={{ minWidth: 0, flex: 1 }}>
                                                    <IconFile size={16} stroke={1.5} />
                                                    <Text size="sm" truncate style={{ flex: 1 }}>{doc.name}</Text>
                                                    <Text size="xs" c="dimmed">{formatFileSize(doc.fileSize)}</Text>
                                                    {doc.indexingStatus === "indexed" && doc.chunkCount > 0 && (
                                                        <Text size="xs" c="dimmed">{t("kb.chunkCount", { count: doc.chunkCount })}</Text>
                                                    )}
                                                    <Badge size="xs" variant="light" color={indexingStatusColor(doc.indexingStatus)}
                                                        leftSection={doc.indexingStatus === "indexing" ? <Loader size={8} /> : undefined}
                                                    >
                                                        {t(`kb.indexing${doc.indexingStatus.charAt(0).toUpperCase() + doc.indexingStatus.slice(1)}`)}
                                                    </Badge>
                                                    {doc.indexingStatus === "failed" && doc.indexingError && (
                                                        <Tooltip label={doc.indexingError}>
                                                            <Text size="xs" c="red" style={{ cursor: "help" }}>(?)</Text>
                                                        </Tooltip>
                                                    )}
                                                </Group>
                                                <Group gap={4}>
                                                    {(doc.indexingStatus === "failed" || doc.indexingStatus === "indexed") && (
                                                        <Tooltip label={t("kb.reindex")}>
                                                            <ActionIcon variant="subtle" size="sm" onClick={() => activeKbId && indexKbDocument(activeKbId, doc.id)}>
                                                                <IconRefresh size={14} stroke={1.5} />
                                                            </ActionIcon>
                                                        </Tooltip>
                                                    )}
                                                    <Tooltip label={t("kb.removeDocument")}>
                                                        <ActionIcon variant="subtle" size="sm" color="red" onClick={() => setDeleteDocId(doc.id)}>
                                                            <IconTrash size={14} stroke={1.5} />
                                                        </ActionIcon>
                                                    </Tooltip>
                                                </Group>
                                            </Group>
                                        ))}
                                    </Stack>
                                )}

                                {/* Settings (collapsible) */}
                                <Group
                                    gap="xs"
                                    style={{ cursor: "pointer" }}
                                    onClick={() => setSettingsOpen(!settingsOpen)}
                                >
                                    {settingsOpen ? <IconChevronDown size={16} /> : <IconChevronRight size={16} />}
                                    <Text fw={600} size="sm">{t("kb.settings")}</Text>
                                </Group>
                                <Collapse in={settingsOpen}>
                                    <Stack gap="sm" pl="md">
                                        <Select
                                            label={t("kb.embeddingModelSelect")}
                                            value={editEmbeddingModel}
                                            onChange={(v) => { setEditEmbeddingModel(v ?? "e5-small"); markDirty(); }}
                                            data={[
                                                { value: "e5-small", label: t("kb.embeddingE5Small") },
                                                { value: "openai", label: t("kb.embeddingOpenai") },
                                                { value: "gemini", label: t("kb.embeddingGemini") },
                                            ]}
                                        />
                                        {editEmbeddingModel !== (activeKb?.embeddingModel ?? "e5-small") && activeKb && activeKb.totalChunks > 0 && (
                                            <Text size="xs" c="orange">{t("kb.embeddingModelChanged")}</Text>
                                        )}
                                        <Select
                                            label={t("kb.chunkingStrategy")}
                                            value={editChunkingStrategy}
                                            onChange={(v) => { setEditChunkingStrategy(v ?? "auto"); markDirty(); }}
                                            data={[
                                                { value: "auto", label: t("kb.chunkingAuto") },
                                                { value: "markdown", label: t("kb.chunkingMarkdown") },
                                                { value: "code", label: t("kb.chunkingCode") },
                                                { value: "html", label: t("kb.chunkingHtml") },
                                                { value: "plain", label: t("kb.chunkingPlain") },
                                                { value: "headings", label: t("kb.chunkingHeadings") },
                                                { value: "paragraphs", label: t("kb.chunkingParagraphs") },
                                                { value: "tokens", label: t("kb.chunkingTokens") },
                                            ]}
                                        />
                                        <NumberInput
                                            label={t("kb.chunkSize")}
                                            value={editChunkSize}
                                            onChange={(v) => { setEditChunkSize(typeof v === "number" ? v : 512); markDirty(); }}
                                            min={64}
                                            max={4096}
                                            step={64}
                                        />
                                        <NumberInput
                                            label={t("kb.chunkOverlap")}
                                            value={editChunkOverlap}
                                            onChange={(v) => { setEditChunkOverlap(typeof v === "number" ? v : 50); markDirty(); }}
                                            min={0}
                                            max={512}
                                            step={10}
                                        />
                                        <NumberInput
                                            label={t("kb.minChunkSize")}
                                            description={t("kb.minChunkSizeTooltip")}
                                            value={editMinChunkSize}
                                            onChange={(v) => { setEditMinChunkSize(typeof v === "number" ? v : 50); markDirty(); }}
                                            min={0}
                                            max={200}
                                            step={10}
                                        />
                                        {activeKb && activeKb.totalChunks > 0 && (
                                            editChunkingStrategy !== activeKb.chunkingStrategy ||
                                            editChunkSize !== activeKb.chunkSize ||
                                            editChunkOverlap !== activeKb.chunkOverlap ||
                                            editMinChunkSize !== (activeKb.minChunkSize ?? 50)
                                        ) && (
                                            <Text size="xs" c="orange">{t("kb.settingsChanged")}</Text>
                                        )}
                                        <NumberInput
                                            label={t("kb.retrievalTopK")}
                                            value={editTopK}
                                            onChange={(v) => { setEditTopK(typeof v === "number" ? v : 5); markDirty(); }}
                                            min={1}
                                            max={50}
                                        />
                                        <Box>
                                            <Text size="sm" mb={4}>{t("kb.retrievalMinScore")}: {editMinScore.toFixed(2)}</Text>
                                            <Slider
                                                value={editMinScore}
                                                onChange={(v) => { setEditMinScore(v); markDirty(); }}
                                                min={0}
                                                max={1}
                                                step={0.05}
                                                marks={[
                                                    { value: 0, label: "0" },
                                                    { value: 0.5, label: "0.5" },
                                                    { value: 1, label: "1" },
                                                ]}
                                            />
                                        </Box>

                                        {/* Query Processing */}
                                        <Text fw={600} size="sm" mt="md">{t("kb.queryProcessing")}</Text>
                                        <Switch
                                            label={t("kb.queryRewriting")}
                                            description={t("kb.queryRewritingDesc")}
                                            checked={editQueryRewriting}
                                            onChange={(e) => { setEditQueryRewriting(e.currentTarget.checked); markDirty(); }}
                                        />
                                        <Group gap="xs">
                                            <Switch
                                                label={t("kb.queryDecomposition")}
                                                description={t("kb.queryDecompositionDesc")}
                                                checked={editQueryDecomposition}
                                                onChange={(e) => { setEditQueryDecomposition(e.currentTarget.checked); markDirty(); }}
                                            />
                                            <Badge color="orange" size="xs" variant="light">{t("kb.experimental")}</Badge>
                                        </Group>
                                        {(editQueryRewriting || editQueryDecomposition) && (
                                            <NumberInput
                                                label={t("kb.maxQueryVariants")}
                                                value={editQueryMaxVariants}
                                                onChange={(v) => { setEditQueryMaxVariants(typeof v === "number" ? v : 3); markDirty(); }}
                                                min={1}
                                                max={5}
                                                step={1}
                                            />
                                        )}
                                        <Text size="xs" c="dimmed">{t("kb.queryProcessingNote")}</Text>

                                        {/* Reranking */}
                                        <Text fw={600} size="sm" mt="md">{t("kb.reranking")}</Text>
                                        <Text size="xs" c="dimmed">{t("kb.rerankerDesc")}</Text>
                                        <Select
                                            label={t("kb.rerankerType")}
                                            data={[
                                                { value: "none", label: t("kb.rerankerNone") },
                                                { value: "cohere", label: t("kb.rerankerCohere") },
                                                { value: "jina", label: t("kb.rerankerJina") },
                                            ]}
                                            value={editRerankerType}
                                            onChange={(v) => { setEditRerankerType(v ?? "none"); markDirty(); }}
                                        />
                                        {editRerankerType !== "none" && (
                                            <>
                                                <Text size="xs" c="dimmed">{t("kb.rerankerKeyRequired")}</Text>
                                                <NumberInput
                                                    label={t("kb.overfetchFactor")}
                                                    description={t("kb.overfetchFactorDesc")}
                                                    value={editRerankerOverfetchFactor}
                                                    onChange={(v) => { setEditRerankerOverfetchFactor(typeof v === "number" ? v : 4); markDirty(); }}
                                                    min={2}
                                                    max={10}
                                                    step={1}
                                                />
                                            </>
                                        )}

                                        {/* Context Optimization */}
                                        <Text fw={600} size="sm" mt="md">{t("kb.contextOptimization")}</Text>
                                        <NumberInput
                                            label={t("kb.tokenBudget")}
                                            description={t("kb.tokenBudgetDesc")}
                                            value={editContextTokenBudget}
                                            onChange={(v) => { setEditContextTokenBudget(typeof v === "number" ? v : 4000); markDirty(); }}
                                            min={0}
                                            max={16000}
                                            step={500}
                                        />
                                        <Switch
                                            label={t("kb.sentenceExtraction")}
                                            description={t("kb.sentenceExtractionDesc")}
                                            checked={editContextSentenceExtraction}
                                            onChange={(e) => { setEditContextSentenceExtraction(e.currentTarget.checked); markDirty(); }}
                                            mt="xs"
                                        />
                                        <Switch
                                            label={t("kb.redundancyRemoval")}
                                            description={t("kb.redundancyRemovalDesc")}
                                            checked={editContextRedundancyRemoval}
                                            onChange={(e) => { setEditContextRedundancyRemoval(e.currentTarget.checked); markDirty(); }}
                                            mt="xs"
                                        />

                                        <Textarea
                                            label={t("kb.systemPrompt")}
                                            placeholder={t("kb.systemPromptPlaceholder")}
                                            value={editSystemPrompt}
                                            onChange={(e) => { setEditSystemPrompt(e.currentTarget.value); markDirty(); }}
                                            minRows={3}
                                            maxRows={8}
                                            autosize
                                        />
                                        {dirty && (
                                            <Button size="xs" onClick={handleSave}>{t("common.save")}</Button>
                                        )}
                                        <Button
                                            size="xs"
                                            variant="light"
                                            color="orange"
                                            leftSection={<IconRefresh size={14} stroke={1.5} />}
                                            onClick={() => setReindexKbId(activeKb.id)}
                                        >
                                            {t("kb.reindex")}
                                        </Button>
                                    </Stack>
                                </Collapse>

                                {/* Stats */}
                                <Text fw={600} size="sm">{t("kb.stats")}</Text>
                                <Group gap="lg">
                                    <Box>
                                        <Text size="xs" c="dimmed">{t("kb.totalDocuments")}</Text>
                                        <Text size="sm" fw={500}>{activeKb.documentCount}</Text>
                                    </Box>
                                    <Box>
                                        <Text size="xs" c="dimmed">{t("kb.totalChunks")}</Text>
                                        <Text size="sm" fw={500}>{activeKb.totalChunks}</Text>
                                    </Box>
                                    <Box>
                                        <Text size="xs" c="dimmed">{t("kb.embeddingModel")}</Text>
                                        <Text size="sm" fw={500}>{activeKb.embeddingModel} ({activeKb.embeddingDimensions}d)</Text>
                                    </Box>
                                    <Box>
                                        <Text size="xs" c="dimmed">{t("kb.version")}</Text>
                                        <Text size="sm" fw={500}>{activeKb.version}</Text>
                                    </Box>
                                </Group>

                                {/* Test Search */}
                                <Text fw={600} size="sm" mt="md">{t("kb.search")}</Text>
                                <Group gap="xs">
                                    <TextInput
                                        placeholder={t("kb.searchPlaceholder")}
                                        size="xs"
                                        style={{ flex: 1 }}
                                        value={searchQuery}
                                        onChange={(e) => setSearchQuery(e.currentTarget.value)}
                                        onKeyDown={(e) => {
                                            if (e.key === "Enter" && searchQuery.trim()) {
                                                handleSearch();
                                            }
                                        }}
                                    />
                                    <Button size="xs" onClick={handleSearch} loading={searchLoading} disabled={!searchQuery.trim()}>
                                        {t("common.search")}
                                    </Button>
                                </Group>
                                {searchResults.length > 0 && (
                                    <Stack gap="xs">
                                        <Text size="xs" c="dimmed">{t("kb.searchResults")} ({searchResults.length})</Text>
                                        {searchResults.map((r, i) => (
                                            <Card key={i} padding="xs" withBorder>
                                                <Group gap={6} mb={4}>
                                                    <Text size="xs" fw={500}>{r.documentName}</Text>
                                                    <Badge size="xs" variant="light" color="gray">#{r.chunkIndex + 1}</Badge>
                                                    <Badge size="xs" variant="light" color="brand">{t("kb.relevance")}: {(r.score * 100).toFixed(0)}%</Badge>
                                                </Group>
                                                <Text size="xs" c="dimmed" lineClamp={4}>{r.content}</Text>
                                            </Card>
                                        ))}
                                    </Stack>
                                )}
                                {searchResults.length === 0 && searchQuery.trim() && !searchLoading && searchPerformed && (
                                    <Text size="xs" c="dimmed" ta="center">{t("kb.noResults")}</Text>
                                )}

                                {/* Export & Delete */}
                                <Group mt="md" gap="xs">
                                    <Button variant="light" size="xs" leftSection={<IconDownload size={14} stroke={1.5} />} onClick={() => exportKnowledgeBase(activeKb.id)}>
                                        {t("kb.export")}
                                    </Button>
                                    <Button color="red" variant="light" size="xs" leftSection={<IconTrash size={14} stroke={1.5} />} onClick={() => setDeleteKbId(activeKb.id)}>
                                        {t("kb.delete")}
                                    </Button>
                                </Group>
                            </Stack>
                        </ScrollArea>
                    ) : (
                        <Box ta="center" py="xl" style={{ flex: 1, display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
                            <IconDatabase size={64} stroke={1} style={{ opacity: 0.2 }} />
                            <Text size="sm" c="dimmed" mt="md">
                                {knowledgeBases.length > 0 ? t("kb.noKbs") : t("kb.noKbsHint")}
                            </Text>
                        </Box>
                    )}
                </Box>
            </Box>

            {/* Modals */}
            <CreateKnowledgeBaseModal opened={createModalOpen} onClose={() => setCreateModalOpen(false)} />
            <ConfirmModal
                opened={deleteKbId !== null}
                onClose={() => setDeleteKbId(null)}
                onConfirm={handleDeleteKb}
                title={t("kb.delete")}
                message={t("kb.deleteConfirm", { name: deleteKbName })}
            />
            <ConfirmModal
                opened={deleteDocId !== null}
                onClose={() => setDeleteDocId(null)}
                onConfirm={handleDeleteDoc}
                title={t("kb.removeDocument")}
                message={t("kb.removeDocumentConfirm", { name: deleteDocName })}
            />
            <ConfirmModal
                opened={reindexKbId !== null}
                onClose={() => setReindexKbId(null)}
                onConfirm={handleReindex}
                title={t("kb.reindex")}
                message={t("kb.reindexConfirm", { name: reindexKbName })}
            />
        </Box>
    );
}

function KbCard({ kb, active, onClick, onDelete, t }: {
    kb: KnowledgeBase;
    active: boolean;
    onClick: () => void;
    onDelete: () => void;
    t: (key: string) => string;
}) {
    return (
        <Card
            shadow={active ? "sm" : undefined}
            padding="sm"
            withBorder
            style={{
                cursor: "pointer",
                borderColor: active ? "var(--mantine-color-brand-filled)" : undefined,
            }}
            onClick={onClick}
        >
            <Group justify="space-between" wrap="nowrap">
                <Box style={{ minWidth: 0, flex: 1 }}>
                    <Text size="sm" fw={600} truncate>{kb.name}</Text>
                    {kb.description && (
                        <Text size="xs" c="dimmed" lineClamp={2}>{kb.description}</Text>
                    )}
                </Box>
                <ActionIcon
                    variant="subtle"
                    size="sm"
                    color="red"
                    onClick={(e) => { e.stopPropagation(); onDelete(); }}
                >
                    <IconTrash size={14} stroke={1.5} />
                </ActionIcon>
            </Group>
            <Group gap="xs" mt={4}>
                <Badge size="xs" variant="light">{kb.documentCount} {t("kb.totalDocuments").toLowerCase()}</Badge>
                <Badge size="xs" variant="light" color={kb.status === "active" ? "green" : "gray"}>
                    {kb.status === "active" ? t("kb.statusActive") : t("kb.statusArchived")}
                </Badge>
            </Group>
        </Card>
    );
}
