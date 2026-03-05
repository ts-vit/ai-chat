import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
    ActionIcon,
    Badge,
    Box,
    Group,
    Loader,
    Paper,
    ScrollArea,
    Text,
    Textarea,
    Tooltip,
} from "@mantine/core";
import { IconFile, IconFileText, IconMicrophone, IconPaperclip, IconPlayerStop, IconX } from "@tabler/icons-react";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../store/chatStore";
import { getUniqueVariableNames } from "./VariablesModal";
import { VariablesModal } from "./VariablesModal";
import { notify } from "../utils/notify";
import { countTokens, formatTokenCount } from "../utils/tokenCount";

const SLASH_POPUP_MAX_ITEMS = 6;
const SLASH_POPUP_ITEM_HEIGHT = 44;

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
const ACCEPT_ATTR = "image/jpeg,image/png,image/gif,image/webp,application/pdf,text/plain,text/markdown,text/csv";
const MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024;

interface Props {
    onSend: (content: string) => void;
    onStop: () => void;
    disabled: boolean;
    isStopping: boolean;
    compact?: boolean;
    /** Optional ref for the message textarea (e.g. for Ctrl+/ focus). */
    inputRef?: React.RefObject<HTMLTextAreaElement | null>;
    contextLength?: number;
    usedTokens?: number;
}

export function MessageInput({ onSend, onStop, disabled, isStopping, compact = false, inputRef, contextLength, usedTokens }: Props) {
    const { t } = useTranslation();
    const [value, setValue] = useState("");
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");
    const [inputTokens, setInputTokens] = useState(0);

    const insertSnippetText = useChatStore((s) => s.insertSnippetText);
    const setInsertSnippetText = useChatStore((s) => s.setInsertSnippetText);
    const snippets = useChatStore((s) => s.snippets);
    const categories = useChatStore((s) => s.categories);
    const settings = useChatStore((s) => s.settings);
    const models = useChatStore((s) => s.models);
    const draftAttachments = useChatStore((s) => s.draftAttachments);
    const addAttachment = useChatStore((s) => s.addAttachment);
    const removeAttachment = useChatStore((s) => s.removeAttachment);

    const [isRecording, setIsRecording] = useState(false);
    const [recordingDuration, setRecordingDuration] = useState(0);
    const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

    const supportsVision = useMemo(
        () => models.find((m) => m.id === settings.model)?.supportsVision ?? false,
        [models, settings.model]
    );
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [isDragging, setIsDragging] = useState(false);

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
            const isImage = (mime: string) => mime.startsWith("image/");
            const processFile = (file: File): Promise<void> => {
                return new Promise((resolve) => {
                    if (!ALLOWED_MIME.includes(file.type)) {
                        notify.error(t("messageInput.fileTypeNotSupported", { name: file.name }));
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
                for (let i = 0; i < fileList.length; i++) {
                    await processFile(fileList[i]);
                }
                if (fileInputRef.current) fileInputRef.current.value = "";
            })();
        },
        [supportsVision, addAttachment]
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

    return (
        <Box
            p={compact ? "xs" : "md"}
            style={{
                position: "relative",
                borderTop: "1px solid var(--mantine-color-default-border)",
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
                        left: 12,
                        right: 52,
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
                accept={ACCEPT_ATTR}
                multiple
                style={{ display: "none" }}
                onChange={(e) => {
                    handleFiles(e.target.files);
                    e.target.value = "";
                }}
            />
            {draftAttachments.length > 0 && (
                <Group gap="xs" wrap="wrap" mb="xs">
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
            <Group wrap="nowrap" align="flex-end">
                <Textarea
                    ref={inputRef}
                    className="message-input-textarea"
                    placeholder={compact ? t("messageInput.placeholderShort") : t("messageInput.placeholderLong")}
                    value={value}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    disabled={disabled}
                    autosize
                    minRows={1}
                    maxRows={6}
                    style={{
                        flex: 1,
                        borderColor: isDragging ? "var(--mantine-color-blue-4)" : undefined,
                        transition: "border-color 150ms ease",
                    }}
                />
                <Tooltip label={supportsVision ? t("messageInput.attachFile") : t("messageInput.modelNoFiles")}>
                    <ActionIcon
                        size="lg"
                        variant="subtle"
                        disabled={!supportsVision}
                        onClick={() => supportsVision && fileInputRef.current?.click()}
                        aria-label={supportsVision ? t("messageInput.attachFile") : t("messageInput.modelNoFiles")}
                    >
                        <IconPaperclip size={18} stroke={1.5} />
                    </ActionIcon>
                </Tooltip>
                <Tooltip label={isRecording ? t("messageInput.stopRecording") : t("messageInput.voiceInput")}>
                    <ActionIcon
                        size="lg"
                        variant="subtle"
                        color={isRecording ? "red" : undefined}
                        className={isRecording ? "recording-pulse" : undefined}
                        disabled={disabled && !isRecording}
                        onClick={handleMicClick}
                        aria-label={isRecording ? t("messageInput.stopRecording") : t("messageInput.voiceInput")}
                    >
                        {isRecording ? (
                            <IconPlayerStop size={18} stroke={1.5} />
                        ) : (
                            <IconMicrophone size={18} stroke={1.5} />
                        )}
                    </ActionIcon>
                </Tooltip>
                {isRecording && (
                    <Text size="xs" c="dimmed" fw={500} style={{ whiteSpace: "nowrap", userSelect: "none" }}>
                        {Math.floor(recordingDuration / 60)}:{String(recordingDuration % 60).padStart(2, "0")}
                    </Text>
                )}
                {disabled ? (
                    <Tooltip label={t("messageInput.stop")}>
                        <ActionIcon
                            size="lg"
                            variant="filled"
                            color="red"
                            onClick={onStop}
                            disabled={isStopping}
                        >
                            {isStopping ? <Loader size="xs" /> : <IconPlayerStop size={18} stroke={1.5} />}
                        </ActionIcon>
                    </Tooltip>
                ) : (
                    <Tooltip label={t("messageInput.send")}>
                        <ActionIcon
                            size="lg"
                            variant="filled"
                            onClick={handleSend}
                            disabled={!value.trim() && draftAttachments.length === 0}
                        >
                            ➤
                        </ActionIcon>
                    </Tooltip>
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
            </Group>
            <VariablesModal
                content={variablesModalContent}
                opened={variablesModalOpen}
                onClose={() => setVariablesModalOpen(false)}
                onSubmit={handleVariablesSubmit}
            />
        </Box>
    );
}