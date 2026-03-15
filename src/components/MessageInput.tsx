import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Button,
    Group,
    Loader,
    Menu,
    Modal,
    Paper,
    Popover,
    ScrollArea,
    Select,
    Stack,
    Text,
    Textarea,
    Tooltip,
    UnstyledButton,
} from "@mantine/core";
import { IconAdjustments, IconArrowUp, IconCheck, IconCode, IconDownload, IconFile, IconFileText, IconMessage2, IconMicrophone, IconPaperclip, IconPlayerStop, IconPlus, IconSubtask, IconWand, IconWorldSearch, IconX } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../store/chatStore";
import { getUniqueVariableNames } from "./VariablesModal";
import { VariablesModal } from "./VariablesModal";
import { ChatParamsPopoverContent } from "./ChatParamsPopover";
import { notify } from "../utils/notify";
import { hasInjections } from "../utils/injections";
import { countTokens, formatTokenCount } from "../utils/tokenCount";
import type { Chat } from "../types";

const SLASH_POPUP_MAX_ITEMS = 6;
const SLASH_POPUP_ITEM_HEIGHT = 44;
const CUSTOM_PROMPT_VALUE = "__custom__";

const ALLOWED_MIME = [
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "application/pdf",
    "text/plain",
    "text/markdown",
    "text/csv",
];
const IMAGE_ONLY_MIME = ["image/jpeg", "image/png", "image/webp"];
const ACCEPT_ATTR = "image/jpeg,image/png,image/gif,image/webp,application/pdf,text/plain,text/markdown,text/csv";
const ACCEPT_IMAGE_ONLY = "image/jpeg,image/png,image/webp";
const MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024;

interface Props {
    onSend: (content: string) => void;
    onStop: () => void;
    disabled: boolean;
    isStopping: boolean;
    compact?: boolean;
    centered?: boolean;
    /** Optional ref for the message textarea (e.g. for Ctrl+/ focus). */
    inputRef?: React.RefObject<HTMLTextAreaElement | null>;
    contextLength?: number;
    usedTokens?: number;
    /** Override vision support check (e.g. for comparison mode). */
    enableAttachments?: boolean;
    /** Active chat for header controls (model, system prompt, params, export). */
    activeChat?: Chat;
}

export function MessageInput({ onSend, onStop, disabled, isStopping, compact = false, centered = false, inputRef, contextLength, usedTokens, enableAttachments, activeChat }: Props) {
    const { t } = useTranslation();
    const [value, setValue] = useState("");
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");
    const [inputTokens, setInputTokens] = useState(0);
    const showInjectionsHint = hasInjections(value);

    const insertSnippetText = useChatStore((s) => s.insertSnippetText);
    const setInsertSnippetText = useChatStore((s) => s.setInsertSnippetText);
    const snippets = useChatStore((s) => s.snippets);
    const categories = useChatStore((s) => s.categories);
    const settings = useChatStore((s) => s.settings);
    const models = useChatStore((s) => s.models);
    const draftAttachments = useChatStore((s) => s.draftAttachments);
    const addAttachment = useChatStore((s) => s.addAttachment);
    const removeAttachment = useChatStore((s) => s.removeAttachment);
    const webSearchEnabled = useChatStore((s) => s.webSearchEnabled);
    const isSearching = useChatStore((s) => s.isSearching);
    const toggleWebSearch = useChatStore((s) => s.toggleWebSearch);
    const presets = useChatStore((s) => s.presets);
    const setChatSystemPrompt = useChatStore((s) => s.setChatSystemPrompt);
    const updateChatModel = useChatStore((s) => s.updateChatModel);
    const updateChatParams = useChatStore((s) => s.updateChatParams);
    const customProviders = useChatStore((s) => s.customProviders);
    const localOllamaModels = useChatStore((s) => s.localOllamaModels);
    const loadLocalOllamaModels = useChatStore((s) => s.loadLocalOllamaModels);
    const skills = useChatStore((s) => s.skills);
    const chatSkills = useChatStore((s) => s.chatSkills);
    const loadChatSkills = useChatStore((s) => s.loadChatSkills);
    const attachSkillToChat = useChatStore((s) => s.attachSkillToChat);
    const detachSkillFromChat = useChatStore((s) => s.detachSkillFromChat);

    const activeMode = useChatStore((s) => s.activeMode);
    const generatePlan = useChatStore((s) => s.generatePlan);
    const planGenerating = useChatStore((s) => s.planGenerating);
    const activeChatId = useChatStore((s) => s.activeChatId);

    const enabledSkills = useMemo(() => skills.filter((s) => s.enabled), [skills]);
    const activeChatSkills = chatSkills;
    const chatMode = activeChat?.mode ?? "chat";

    useEffect(() => {
        if (activeChat?.id) {
            loadChatSkills(activeChat.id);
        }
    }, [activeChat?.id, loadChatSkills]);

    const [isRecording, setIsRecording] = useState(false);
    const [recordingDuration, setRecordingDuration] = useState(0);
    const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

    // Header controls state
    const [paramsPopoverOpen, setParamsPopoverOpen] = useState(false);
    const [modelPopoverOpen, setModelPopoverOpen] = useState(false);
    const [customModalOpen, setCustomModalOpen] = useState(false);
    const [customPromptDraft, setCustomPromptDraft] = useState("");
    const [customProviderModelsCache, setCustomProviderModelsCache] = useState<Record<string, string[]>>({});
    const [customModelsLoading, setCustomModelsLoading] = useState(false);

    const currentModel = useMemo(() => models.find((m) => m.id === settings.model), [models, settings.model]);
    const isImageModel = currentModel?.supportsImageGeneration ?? false;
    const supportsVision = useMemo(
        () => enableAttachments ?? (currentModel?.supportsVision ?? isImageModel),
        [currentModel?.supportsVision, isImageModel, enableAttachments]
    );
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [isDragging, setIsDragging] = useState(false);

    // System prompt logic
    const currentSystemPrompt = activeChat?.systemPrompt ?? "";
    const matchingPreset = currentSystemPrompt
        ? presets.find((p) => p.content === currentSystemPrompt)
        : null;
    const selectValue = customModalOpen
        ? CUSTOM_PROMPT_VALUE
        : matchingPreset
          ? matchingPreset.id
          : currentSystemPrompt
            ? CUSTOM_PROMPT_VALUE
            : "";

    const presetSelectData = useMemo(() => [
        { value: "", label: t("chat.noPrompt") },
        ...presets.map((p) => ({ value: p.id, label: p.name })),
        { value: CUSTOM_PROMPT_VALUE, label: t("chat.customPrompt") },
    ], [presets, t]);

    const currentPresetLabel = useMemo(() => {
        if (!currentSystemPrompt) return t("chat.noPrompt");
        if (matchingPreset) return matchingPreset.name;
        return t("chat.customPrompt");
    }, [currentSystemPrompt, matchingPreset, t]);

    // Model selection logic
    const providerId = activeChat?.providerId ?? "openrouter";
    const isCustomProvider = providerId !== "openrouter" && providerId !== "ollama";

    useEffect(() => {
        if (providerId === "ollama") {
            loadLocalOllamaModels();
        }
    }, [providerId, loadLocalOllamaModels]);

    useEffect(() => {
        if (!isCustomProvider || !providerId) return;
        if (customProviderModelsCache[providerId] !== undefined) return;
        const provider = customProviders.find((p) => p.id === providerId);
        if (!provider) return;
        setCustomModelsLoading(true);
        invoke<Array<{ id: string; name: string }>>(
            "fetch_custom_provider_models",
            {
                baseUrl: provider.baseUrl,
                apiKey: provider.apiKey ?? "",
            }
        )
            .then((list) => {
                const ids = Array.isArray(list) ? list.map((m) => m.id) : [];
                setCustomProviderModelsCache((prev) => ({ ...prev, [providerId]: ids }));
            })
            .catch((e) => {
                notify.error(String(e));
                setCustomProviderModelsCache((prev) => ({ ...prev, [providerId]: [] }));
            })
            .finally(() => setCustomModelsLoading(false));
    }, [isCustomProvider, providerId, customProviders, customProviderModelsCache]);

    const modelSelectData = useMemo(() => {
        if (providerId === "openrouter") {
            const openRouterModels = settings.openrouterEnabledModels ?? [];
            return openRouterModels.map((id) => ({ value: id, label: id }));
        }
        if (providerId === "ollama") {
            return localOllamaModels.map((m) => ({ value: m.name, label: m.name }));
        }
        if (isCustomProvider) {
            const ids = customProviderModelsCache[providerId] ?? [];
            return ids.map((id) => ({ value: id, label: id }));
        }
        return [];
    }, [providerId, isCustomProvider, settings.openrouterEnabledModels, localOllamaModels, customProviderModelsCache]);

    const handleModelChange = useCallback((value: string | null) => {
        if (!activeChat?.id || value === null || value === "") return;
        const modelInfo = models.find((m) => m.id === value);
        updateChatModel(activeChat.id, value, modelInfo?.supportsImageGeneration ?? false);
        setModelPopoverOpen(false);
    }, [activeChat?.id, models, updateChatModel]);

    const handlePresetChange = useCallback((value: string | null) => {
        if (!activeChat?.id) return;
        if (value === null || value === "") {
            setChatSystemPrompt(activeChat.id, "");
            return;
        }
        if (value === CUSTOM_PROMPT_VALUE) {
            setCustomPromptDraft(currentSystemPrompt);
            setCustomModalOpen(true);
            return;
        }
        const preset = presets.find((p) => p.id === value);
        if (preset) setChatSystemPrompt(activeChat.id, preset.content);
    }, [activeChat?.id, presets, setChatSystemPrompt, currentSystemPrompt]);

    const saveCustomPrompt = useCallback(() => {
        if (activeChat?.id) {
            setChatSystemPrompt(activeChat.id, customPromptDraft);
        }
        setCustomModalOpen(false);
    }, [activeChat?.id, setChatSystemPrompt, customPromptDraft]);

    useEffect(() => {
        if (insertSnippetText !== null) {
            setValue(insertSnippetText);
            setInsertSnippetText(null);
        }
    }, [insertSnippetText, setInsertSnippetText]);

    useEffect(() => {
        const timer = setTimeout(async () => {
            const count = await countTokens(value);
            setInputTokens(count);
        }, 500);
        return () => clearTimeout(timer);
    }, [value]);

    const showSlashPopup = value.startsWith("/");
    const slashQuery = value.slice(1).trim().toLowerCase();

    const filteredSnippets = useMemo(() => {
        if (!showSlashPopup) return [];
        if (!slashQuery) return snippets.slice(0, 20);
        return snippets.filter(
            (s) =>
                s.name.toLowerCase().includes(slashQuery) ||
                (categories.find((c) => c.id === s.categoryId)?.name ?? "")
                    .toLowerCase()
                    .includes(slashQuery)
        );
    }, [showSlashPopup, slashQuery, snippets, categories]);

    useEffect(() => {
        setSelectedIndex((i) =>
            filteredSnippets.length ? Math.min(i, filteredSnippets.length - 1) : 0
        );
    }, [filteredSnippets.length]);

    const getCategoryName = useCallback(
        (categoryId: string) => categories.find((c) => c.id === categoryId)?.name ?? categoryId,
        [categories]
    );

    const applySnippet = useCallback(
        (snippet: (typeof snippets)[0]) => {
            const vars = getUniqueVariableNames(snippet.content);
            if (vars.length === 0) {
                setValue(snippet.content);
                return;
            }
            setVariablesModalContent(snippet.content);
            setVariablesModalOpen(true);
        },
        []
    );

    const handleVariablesSubmit = useCallback((filledText: string) => {
        setValue(filledText);
        setVariablesModalOpen(false);
    }, []);

    const handleSend = () => {
        const trimmed = value.trim();
        if (!trimmed || disabled) return;
        // Intercept /plan command in assistant mode
        if (trimmed.startsWith("/plan ") && activeMode === "assistant" && activeChatId) {
            const goal = trimmed.slice(6).trim();
            if (goal) {
                generatePlan(activeChatId, goal);
                setValue("");
                return;
            }
        }
        onSend(trimmed);
        setValue("");
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (showSlashPopup && filteredSnippets.length > 0) {
            if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelectedIndex((i) => (i + 1) % filteredSnippets.length);
                return;
            }
            if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelectedIndex((i) =>
                    i <= 0 ? filteredSnippets.length - 1 : i - 1
                );
                return;
            }
            if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                applySnippet(filteredSnippets[selectedIndex]);
                return;
            }
            if (e.key === "Escape") {
                e.preventDefault();
                setValue("");
                return;
            }
        }
        if (e.key === "Enter") {
            const sendByEnter = settings.sendByEnter !== false;
            if (sendByEnter) {
                if (!e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                }
            } else {
                if (e.ctrlKey || e.metaKey) {
                    e.preventDefault();
                    handleSend();
                }
            }
        }
    };

    const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        setValue(e.currentTarget.value);
    };

    const handleFiles = useCallback(
        (fileList: FileList | null) => {
            if (!fileList?.length || !supportsVision) return;
            const allowedMime = isImageModel ? IMAGE_ONLY_MIME : ALLOWED_MIME;
            if (isImageModel && draftAttachments.length >= 1) {
                notify.error(t("messageInput.maxOneImage"));
                return;
            }
            const isImage = (mime: string) => mime.startsWith("image/");
            const processFile = (file: File): Promise<void> => {
                return new Promise((resolve) => {
                    if (!allowedMime.includes(file.type)) {
                        notify.error(
                            isImageModel
                                ? t("messageInput.onlyImagesAllowed")
                                : t("messageInput.fileTypeNotSupported", { name: file.name })
                        );
                        resolve();
                        return;
                    }
                    if (file.size > MAX_FILE_SIZE_BYTES) {
                        notify.error(t("messageInput.fileTooBig", { name: file.name }));
                        resolve();
                        return;
                    }
                    const reader = new FileReader();
                    reader.onload = () => {
                        const buf = reader.result as ArrayBuffer;
                        const arr = new Uint8Array(buf);
                        const data = Array.from(arr);
                        let previewUrl = "";
                        if (isImage(file.type)) {
                            const dataReader = new FileReader();
                            dataReader.onload = () => {
                                previewUrl = dataReader.result as string;
                                addAttachment({
                                    id: crypto.randomUUID(),
                                    name: file.name,
                                    mimeType: file.type,
                                    size: file.size,
                                    previewUrl,
                                    data,
                                });
                                resolve();
                            };
                            dataReader.readAsDataURL(file);
                        } else {
                            addAttachment({
                                id: crypto.randomUUID(),
                                name: file.name,
                                mimeType: file.type,
                                size: file.size,
                                previewUrl: "",
                                data,
                            });
                            resolve();
                        }
                    };
                    reader.readAsArrayBuffer(file);
                });
            };
            (async () => {
                const maxFiles = isImageModel ? 1 : fileList.length;
                for (let i = 0; i < Math.min(fileList.length, maxFiles); i++) {
                    await processFile(fileList[i]);
                }
                if (isImageModel && fileList.length > 1) {
                    notify.error(t("messageInput.maxOneImage"));
                }
                if (fileInputRef.current) fileInputRef.current.value = "";
            })();
        },
        [supportsVision, isImageModel, draftAttachments.length, addAttachment, t]
    );

    useEffect(() => {
        return () => {
            if (recordingTimerRef.current) clearInterval(recordingTimerRef.current);
        };
    }, []);

    const handleMicClick = useCallback(async () => {
        if (isRecording) {
            setIsRecording(false);
            if (recordingTimerRef.current) {
                clearInterval(recordingTimerRef.current);
                recordingTimerRef.current = null;
            }
            setRecordingDuration(0);
            try {
                const text = await invoke<string>("stop_recording_and_transcribe", {
                    provider: settings.sttProvider,
                    language: settings.sttLanguage,
                });
                if (text.trim()) {
                    setValue((prev) => (prev ? `${prev} ${text}` : text));
                }
            } catch (err) {
                notify.error(String(err));
            }
        } else {
            if (!settings.sttProvider) {
                notify.error(t("messageInput.sttNotConfigured"));
                return;
            }
            if (settings.sttProvider === "whisper" && !settings.openaiApiKey?.trim()) {
                notify.error(t("messageInput.sttOpenAiKeyNotSet"));
                return;
            }
            if (settings.sttProvider === "groq" && !settings.groqSttApiKey?.trim()) {
                notify.error(t("messageInput.sttGroqKeyNotSet"));
                return;
            }
            try {
                await invoke("start_recording");
                setIsRecording(true);
                setRecordingDuration(0);
                recordingTimerRef.current = setInterval(() => {
                    setRecordingDuration((d) => d + 1);
                }, 1000);
            } catch (err) {
                notify.error(String(err));
            }
        }
    }, [isRecording, settings.sttProvider, settings.sttLanguage, settings.openaiApiKey, settings.groqSttApiKey, t]);

    const handleDragOver = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(true);
    }, []);
    const handleDragLeave = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);
    }, []);
    const handleDrop = useCallback(
        (e: React.DragEvent) => {
            e.preventDefault();
            setIsDragging(false);
            if (!supportsVision) {
                notify.warning(t("messageInput.modelNoFiles"));
                return;
            }
            handleFiles(e.dataTransfer.files);
        },
        [supportsVision, handleFiles]
    );

    const hasContent = value.trim().length > 0 || draftAttachments.length > 0;

    return (
        <Box
            p={compact ? "xs" : "md"}
            style={{
                position: "relative",
                ...(centered ? { maxWidth: 680, width: "100%", margin: "0 auto" } : {}),
            }}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
        >
            {showSlashPopup && filteredSnippets.length > 0 && (
                <Paper
                    shadow="md"
                    p={0}
                    withBorder
                    radius="md"
                    style={{
                        position: "absolute",
                        bottom: "100%",
                        left: 16,
                        right: 16,
                        maxHeight: SLASH_POPUP_MAX_ITEMS * SLASH_POPUP_ITEM_HEIGHT,
                        overflow: "hidden",
                        zIndex: 100,
                        marginBottom: 4,
                    }}
                >
                    <ScrollArea.Autosize mah={SLASH_POPUP_MAX_ITEMS * SLASH_POPUP_ITEM_HEIGHT}>
                        {filteredSnippets.map((s, i) => (
                            <Box
                                key={s.id}
                                style={{
                                    padding: "8px 12px",
                                    cursor: "pointer",
                                    backgroundColor:
                                        i === selectedIndex
                                            ? "var(--mantine-color-default-hover)"
                                            : undefined,
                                }}
                                onClick={() => applySnippet(s)}
                                onMouseEnter={() => setSelectedIndex(i)}
                            >
                                <Group gap="xs" wrap="nowrap">
                                    <Text size="sm" fw={500} lineClamp={1} style={{ flex: 1 }}>
                                        {s.name}
                                    </Text>
                                    <Badge size="xs" variant="outline">
                                        {getCategoryName(s.categoryId)}
                                    </Badge>
                                </Group>
                            </Box>
                        ))}
                    </ScrollArea.Autosize>
                </Paper>
            )}
            <input
                ref={fileInputRef}
                type="file"
                accept={isImageModel ? ACCEPT_IMAGE_ONLY : ACCEPT_ATTR}
                multiple={!isImageModel}
                style={{ display: "none" }}
                onChange={(e) => {
                    handleFiles(e.target.files);
                    e.target.value = "";
                }}
            />

            {isSearching && (
                <Group gap="xs" mb={8} justify="center">
                    <Loader size="xs" />
                    <Text size="xs" c="dimmed">{t("messageInput.searching")}</Text>
                </Group>
            )}

            <Box className={`message-input-pill ${isDragging ? "dragging" : ""}`}>
                {/* Model name above textarea */}
                {activeChat && (
                    <Popover
                        width={300}
                        position="top-start"
                        shadow="md"
                        opened={modelPopoverOpen}
                        onChange={setModelPopoverOpen}
                    >
                        <Popover.Target>
                            <UnstyledButton
                                onClick={() => setModelPopoverOpen((o) => !o)}
                                style={{ display: "block", padding: "4px 12px 0" }}
                            >
                                <Text size="xs" c="dimmed" style={{ cursor: "pointer" }}>
                                    {activeChat.model || t("chat.selectModel")}
                                </Text>
                            </UnstyledButton>
                        </Popover.Target>
                        <Popover.Dropdown>
                            <Select
                                size="xs"
                                data={modelSelectData}
                                value={activeChat.model ?? ""}
                                onChange={handleModelChange}
                                placeholder={
                                    customModelsLoading && isCustomProvider
                                        ? t("common.loading")
                                        : t("chat.selectModel")
                                }
                                searchable
                                allowDeselect={false}
                                disabled={customModelsLoading && isCustomProvider}
                            />
                        </Popover.Dropdown>
                    </Popover>
                )}

                {draftAttachments.length > 0 && (
                    <Group gap="xs" wrap="wrap" mb={8}>
                        {draftAttachments.map((a) => (
                            <Box
                                key={a.id}
                                style={{
                                    position: "relative",
                                    width: 64,
                                    height: 64,
                                    borderRadius: "var(--mantine-radius-sm)",
                                    overflow: "hidden",
                                    backgroundColor: "var(--mantine-color-default-hover)",
                                    flexShrink: 0,
                                }}
                            >
                                {a.previewUrl ? (
                                    <img
                                        src={a.previewUrl}
                                        alt={a.name}
                                        style={{
                                            width: "100%",
                                            height: "100%",
                                            objectFit: "cover",
                                        }}
                                    />
                                ) : (
                                    <Box
                                        style={{
                                            width: "100%",
                                            height: "100%",
                                            display: "flex",
                                            flexDirection: "column",
                                            alignItems: "center",
                                            justifyContent: "center",
                                            padding: 4,
                                        }}
                                    >
                                        {a.mimeType === "application/pdf" ? (
                                            <IconFile size={24} stroke={1.5} style={{ color: "var(--mantine-color-default-color)" }} />
                                        ) : (
                                            <IconFileText size={24} stroke={1.5} style={{ color: "var(--mantine-color-default-color)" }} />
                                        )}
                                        <Text size="xs" lineClamp={2} ta="center" style={{ marginTop: 2 }}>
                                            {a.name}
                                        </Text>
                                    </Box>
                                )}
                                <Tooltip label={t("common.delete")}>
                                    <ActionIcon
                                        size="xs"
                                        variant="filled"
                                        color="red"
                                        style={{ position: "absolute", top: 2, right: 2 }}
                                        onClick={() => removeAttachment(a.id)}
                                        aria-label={t("common.delete")}
                                    >
                                        <IconX size={12} stroke={1.5} />
                                    </ActionIcon>
                                </Tooltip>
                            </Box>
                        ))}
                    </Group>
                )}

                <Textarea
                    ref={inputRef}
                    placeholder={compact ? t("messageInput.placeholderShort") : t("messageInput.placeholderLong")}
                    value={value}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    disabled={disabled}
                    autosize
                    minRows={1}
                    maxRows={6}
                />

                <Group justify="space-between" align="center" wrap="nowrap" gap={4} mt={4} className="input-toolbar">
                    <Group gap={2} wrap="nowrap">
                        {!isRecording && (
                            <Menu position="top-start" shadow="md" width={220}>
                                <Menu.Target>
                                    <ActionIcon variant="subtle" color="brand" size="sm" radius="xl" aria-label="More options">
                                        <IconPlus size={16} stroke={1.5} />
                                    </ActionIcon>
                                </Menu.Target>
                                <Menu.Dropdown>
                                    <Menu.Item
                                        leftSection={<IconPaperclip size={16} stroke={1.5} />}
                                        onClick={() => supportsVision && fileInputRef.current?.click()}
                                        disabled={!supportsVision}
                                    >
                                        {isImageModel ? t("messageInput.attachImageForEdit") : t("messageInput.attachFile")}
                                    </Menu.Item>
                                    <Menu.Item
                                        leftSection={<IconWorldSearch size={16} stroke={1.5} />}
                                        onClick={() => toggleWebSearch()}
                                        rightSection={webSearchEnabled ? <IconCheck size={14} color="var(--mantine-color-brand-5)" /> : null}
                                    >
                                        {t("messageInput.webSearch")}
                                    </Menu.Item>
                                    {activeMode === "assistant" && activeChatId && (
                                        <Menu.Item
                                            leftSection={<IconSubtask size={16} stroke={1.5} />}
                                            onClick={() => {
                                                const goal = value.trim();
                                                if (goal) {
                                                    generatePlan(activeChatId, goal);
                                                    setValue("");
                                                }
                                            }}
                                            disabled={!value.trim() || planGenerating}
                                        >
                                            {planGenerating ? t("plans.generating") : t("plans.createPlan")}
                                        </Menu.Item>
                                    )}
                                    {activeChat && (
                                        <>
                                            <Menu.Divider />
                                            <Menu.Item
                                                leftSection={<IconDownload size={16} stroke={1.5} />}
                                                onClick={async () => {
                                                    try {
                                                        await invoke("export_chat_json", { chatId: activeChat.id });
                                                        notify.success(t("chat.exportSuccess"));
                                                    } catch (e) {
                                                        notify.error(String(e));
                                                    }
                                                }}
                                            >
                                                {t("chat.exportJson")}
                                            </Menu.Item>
                                            <Menu.Item
                                                leftSection={<IconDownload size={16} stroke={1.5} />}
                                                onClick={async () => {
                                                    try {
                                                        await invoke("export_chat_markdown", { chatId: activeChat.id });
                                                        notify.success(t("chat.exportSuccess"));
                                                    } catch (e) {
                                                        notify.error(String(e));
                                                    }
                                                }}
                                            >
                                                {t("chat.exportMarkdown")}
                                            </Menu.Item>
                                        </>
                                    )}
                                </Menu.Dropdown>
                            </Menu>
                        )}

                        {/* System prompt selector */}
                        {activeChat && !isRecording && (
                            <Menu position="top-start" shadow="md" width={220}>
                                <Menu.Target>
                                    <Tooltip label={t("chat.systemPrompt")}>
                                        <UnstyledButton>
                                            <Group gap={4} wrap="nowrap">
                                                <IconMessage2 size={16} stroke={1.5} style={{ color: "var(--mantine-color-dimmed)" }} />
                                                <Text size="xs" c="dimmed" truncate="end" style={{ maxWidth: 120 }}>
                                                    {currentPresetLabel}
                                                </Text>
                                            </Group>
                                        </UnstyledButton>
                                    </Tooltip>
                                </Menu.Target>
                                <Menu.Dropdown>
                                    {presetSelectData.map((item) => (
                                        <Menu.Item
                                            key={item.value}
                                            onClick={() => handlePresetChange(item.value)}
                                            fw={selectValue === item.value ? 600 : undefined}
                                            color={selectValue === item.value ? "brand" : undefined}
                                        >
                                            {item.label}
                                        </Menu.Item>
                                    ))}
                                </Menu.Dropdown>
                            </Menu>
                        )}

                        {webSearchEnabled && (
                            <Badge
                                size="xs"
                                variant="light"
                                color="brand"
                                radius="sm"
                                style={{ cursor: "pointer" }}
                                onClick={() => toggleWebSearch()}
                            >
                                {t("messageInput.webSearchOn")}
                            </Badge>
                        )}

                        {activeChatSkills.length > 0 && activeChatSkills.map((skill) => (
                            <Badge
                                key={skill.id}
                                size="xs"
                                variant="light"
                                color="brand"
                                radius="sm"
                                style={{ cursor: "pointer" }}
                                onClick={() => activeChat && detachSkillFromChat(activeChat.id, skill.id)}
                            >
                                {skill.name} ×
                            </Badge>
                        ))}

                        {showInjectionsHint && (
                            <Tooltip label={t("messageInput.injectionsTooltip")}>
                                <Badge
                                    size="xs"
                                    variant="light"
                                    color="brand"
                                    radius="sm"
                                    leftSection={<IconCode size={12} stroke={1.5} />}
                                    style={{ cursor: "default" }}
                                >
                                    {t("messageInput.injectionsActive")}
                                </Badge>
                            </Tooltip>
                        )}
                    </Group>

                    <Group gap={4} wrap="nowrap">
                        {isRecording && (
                            <Text size="xs" c="dimmed" fw={500} style={{ whiteSpace: "nowrap", userSelect: "none" }}>
                                {Math.floor(recordingDuration / 60)}:{String(recordingDuration % 60).padStart(2, "0")}
                            </Text>
                        )}

                        {inputTokens > 0 && (() => {
                            const totalTokens = (usedTokens ?? 0) + inputTokens;
                            const hasContext = contextLength != null && contextLength > 0 && usedTokens != null;
                            const exceeded = hasContext && totalTokens > contextLength;
                            const warning = hasContext && !exceeded && totalTokens > contextLength * 0.9;
                            const color = exceeded ? "red" : warning ? "yellow" : "dimmed";
                            const label = formatTokenCount(inputTokens);
                            const tooltipText = exceeded
                                ? t("messageInput.contextExceeded")
                                : warning
                                  ? t("messageInput.contextWarning")
                                  : hasContext
                                    ? t("messageInput.contextUsage", {
                                          used: formatTokenCount(usedTokens),
                                          input: formatTokenCount(inputTokens),
                                          limit: formatTokenCount(contextLength),
                                      })
                                    : undefined;
                            const textEl = (
                                <Text size="xs" c={color} style={{ whiteSpace: "nowrap", userSelect: "none" }}>
                                    {label} tok
                                </Text>
                            );
                            return tooltipText ? <Tooltip label={tooltipText}>{textEl}</Tooltip> : textEl;
                        })()}

                        {/* Parameters popover */}
                        {activeChat && (
                            <Popover
                                width={320}
                                position="top-end"
                                withArrow
                                shadow="md"
                                opened={paramsPopoverOpen}
                                onChange={setParamsPopoverOpen}
                            >
                                <Popover.Target>
                                    <Tooltip label={t("chatParams.title")}>
                                        <ActionIcon
                                            variant="subtle"
                                            size="sm"
                                            onClick={() => setParamsPopoverOpen((o) => !o)}
                                            aria-label={t("chatParams.title")}
                                        >
                                            <IconAdjustments size={16} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                </Popover.Target>
                                <Popover.Dropdown>
                                    <div style={{ maxHeight: 'calc(100vh - 200px)', overflowY: 'auto' }}>
                                        <ChatParamsPopoverContent
                                            chat={activeChat}
                                            settings={settings}
                                            onParamsChange={updateChatParams}
                                        />
                                    </div>
                                </Popover.Dropdown>
                            </Popover>
                        )}

                        {/* Skill selector */}
                        {activeChat && chatMode === "assistant" && (
                            <Menu position="top-end" shadow="md" width={220}>
                                <Menu.Target>
                                    <Tooltip label={t("skills.selectSkill")}>
                                        <ActionIcon variant="subtle" size="sm" color={activeChatSkills.length > 0 ? "brand" : undefined}>
                                            <IconWand size={16} stroke={1.5} />
                                        </ActionIcon>
                                    </Tooltip>
                                </Menu.Target>
                                <Menu.Dropdown>
                                    <Menu.Label>{t("skills.selectSkill")}</Menu.Label>
                                    {enabledSkills.map((skill) => {
                                        const isAttached = activeChatSkills.some((s) => s.id === skill.id);
                                        return (
                                            <Menu.Item
                                                key={skill.id}
                                                rightSection={isAttached ? <IconCheck size={14} color="var(--mantine-color-brand-5)" /> : null}
                                                onClick={() => {
                                                    if (!activeChat) return;
                                                    if (isAttached) {
                                                        detachSkillFromChat(activeChat.id, skill.id);
                                                    } else {
                                                        attachSkillToChat(activeChat.id, skill.id);
                                                    }
                                                }}
                                            >
                                                {skill.name}
                                            </Menu.Item>
                                        );
                                    })}
                                    {activeChatSkills.length > 0 && (
                                        <>
                                            <Menu.Divider />
                                            <Menu.Item
                                                color="red"
                                                onClick={() => {
                                                    if (!activeChat) return;
                                                    activeChatSkills.forEach((s) => detachSkillFromChat(activeChat.id, s.id));
                                                }}
                                            >
                                                {t("skills.detachSkill")}
                                            </Menu.Item>
                                        </>
                                    )}
                                </Menu.Dropdown>
                            </Menu>
                        )}

                        <Tooltip label={isRecording ? t("messageInput.stopRecording") : t("messageInput.voiceInput")}>
                            <ActionIcon
                                size="sm"
                                radius="xl"
                                variant="subtle"
                                color={isRecording ? "red" : "brand"}
                                className={isRecording ? "recording-pulse" : undefined}
                                disabled={disabled && !isRecording}
                                onClick={handleMicClick}
                                aria-label={isRecording ? t("messageInput.stopRecording") : t("messageInput.voiceInput")}
                            >
                                {isRecording ? (
                                    <IconPlayerStop size={16} stroke={1.5} />
                                ) : (
                                    <IconMicrophone size={16} stroke={1.5} />
                                )}
                            </ActionIcon>
                        </Tooltip>

                        {disabled ? (
                            <Tooltip label={t("messageInput.stop")}>
                                <ActionIcon
                                    size="sm"
                                    radius="xl"
                                    variant="filled"
                                    color="red"
                                    onClick={onStop}
                                    disabled={isStopping}
                                >
                                    {isStopping ? <Loader size="xs" /> : <IconPlayerStop size={16} stroke={1.5} />}
                                </ActionIcon>
                            </Tooltip>
                        ) : (
                            <Tooltip label={t("messageInput.send")}>
                                <ActionIcon
                                    size="sm"
                                    radius="xl"
                                    variant={hasContent ? "filled" : "subtle"}
                                    color="brand"
                                    onClick={handleSend}
                                    disabled={!hasContent}
                                >
                                    <IconArrowUp size={16} stroke={2} />
                                </ActionIcon>
                            </Tooltip>
                        )}
                    </Group>
                </Group>
            </Box>

            {/* Custom system prompt modal */}
            <Modal
                title={t("chat.customPromptModalTitle")}
                size="md"
                opened={customModalOpen}
                onClose={() => setCustomModalOpen(false)}
            >
                <Stack gap="sm">
                    <Textarea
                        placeholder={t("chat.customPromptPlaceholder")}
                        value={customPromptDraft}
                        onChange={(e) => setCustomPromptDraft(e.currentTarget.value)}
                        minRows={3}
                        maxRows={10}
                        autosize
                    />
                    <Group justify="flex-end" gap="sm">
                        <Button variant="subtle" onClick={() => setCustomModalOpen(false)}>
                            {t("common.cancel")}
                        </Button>
                        <Button onClick={saveCustomPrompt}>
                            {t("common.save")}
                        </Button>
                    </Group>
                </Stack>
            </Modal>

            <VariablesModal
                content={variablesModalContent}
                opened={variablesModalOpen}
                onClose={() => setVariablesModalOpen(false)}
                onSubmit={handleVariablesSubmit}
            />
        </Box>
    );
}
