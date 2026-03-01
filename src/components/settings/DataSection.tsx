import { useTranslation } from "react-i18next";
import { Button, Divider, Stack, Text } from "@mantine/core";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../../store/chatStore";
import { notify } from "../../utils/notify";
import type { ImportResult } from "../../types";

export function DataSection() {
    const { t } = useTranslation();
    const loadChats = useChatStore((s) => s.loadChats);

    return (
        <Stack gap="lg">
            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.data.backup")}
                </Text>
                <Button
                    variant="light"
                    size="sm"
                    onClick={async () => {
                        try {
                            await invoke("export_all_chats");
                            notify.success(t("notifications.chatsExported"));
                        } catch (e) {
                            notify.error(String(e));
                        }
                    }}
                >
                    {t("settings.data.exportAll")}
                </Button>
                <Text size="xs" c="dimmed">
                    {t("settings.data.exportAllHint")}
                </Text>
            </Stack>

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.data.importTitle")}
                </Text>
                <Button
                    variant="light"
                    size="sm"
                    onClick={async () => {
                        try {
                            const result = await invoke<ImportResult>("import_chats");
                            notify.success(
                                t("notifications.importSuccess", {
                                    chats: result.chatsImported,
                                    messages: result.messagesImported,
                                    attachments: result.attachmentsImported,
                                })
                            );
                            await loadChats();
                        } catch (e) {
                            notify.error(String(e));
                        }
                    }}
                >
                    {t("settings.data.importButton")}
                </Button>
                <Text size="xs" c="dimmed">
                    {t("settings.data.importHint")}
                </Text>
            </Stack>

            <Divider />

            <Stack gap="xs">
                <Text size="sm" fw={500}>
                    {t("settings.data.clearTitle")}
                </Text>
                <Button variant="light" size="sm" color="red" disabled>
                    {t("settings.data.clearButton")}
                </Button>
                <Text size="xs" c="dimmed">
                    {t("settings.data.clearHint")}
                </Text>
            </Stack>
        </Stack>
    );
}
