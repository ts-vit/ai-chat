import { useCallback, useEffect, useMemo, useState } from "react";
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
import { IconPlayerStop } from "@tabler/icons-react";
import { useChatStore } from "../store/chatStore";
import { getUniqueVariableNames } from "./VariablesModal";
import { VariablesModal } from "./VariablesModal";

const SLASH_POPUP_MAX_ITEMS = 6;
const SLASH_POPUP_ITEM_HEIGHT = 44;

interface Props {
    onSend: (content: string) => void;
    onStop: () => void;
    disabled: boolean;
    isStopping: boolean;
    compact?: boolean;
}

export function MessageInput({ onSend, onStop, disabled, isStopping, compact = false }: Props) {
    const [value, setValue] = useState("");
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [variablesModalOpen, setVariablesModalOpen] = useState(false);
    const [variablesModalContent, setVariablesModalContent] = useState("");

    const insertSnippetText = useChatStore((s) => s.insertSnippetText);
    const setInsertSnippetText = useChatStore((s) => s.setInsertSnippetText);
    const snippets = useChatStore((s) => s.snippets);
    const categories = useChatStore((s) => s.categories);

    useEffect(() => {
        if (insertSnippetText !== null) {
            setValue(insertSnippetText);
            setInsertSnippetText(null);
        }
    }, [insertSnippetText, setInsertSnippetText]);

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
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        setValue(e.currentTarget.value);
    };

    return (
        <Box
            p={compact ? "xs" : "md"}
            style={{
                position: "relative",
                borderTop: "1px solid var(--mantine-color-default-border)",
            }}
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
            <Group wrap="nowrap" align="flex-end">
                <Textarea
                    placeholder={compact ? "Сообщение..." : "Напишите сообщение... (наберите / для шаблонов)"}
                    value={value}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    disabled={disabled}
                    autosize
                    minRows={1}
                    maxRows={6}
                    style={{ flex: 1 }}
                />
                {disabled ? (
                    <Tooltip label="Остановить">
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
                    <Tooltip label="Отправить">
                        <ActionIcon
                            size="lg"
                            variant="filled"
                            onClick={handleSend}
                            disabled={!value.trim()}
                        >
                            ➤
                        </ActionIcon>
                    </Tooltip>
                )}
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