import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import {
    Stack,
    Text,
    Button,
    Progress,
    Group,
    Badge,
    Alert,
    Paper,
} from "@mantine/core";
import {
    IconDownload,
    IconRefresh,
    IconAlertTriangle,
    IconTrash,
    IconBrain,
} from "@tabler/icons-react";

interface ModelStatus {
    installed: boolean;
    current_version: string | null;
    latest_version: string | null;
    update_available: boolean;
    model_size_mb: number | null;
    source: string;
}

interface DownloadProgress {
    file_name: string;
    downloaded: number;
    total: number;
    percent: number;
}

export default function SearchModelSection() {
    const { t } = useTranslation();
    const [status, setStatus] = useState<ModelStatus | null>(null);
    const [loading, setLoading] = useState(false);
    const [downloading, setDownloading] = useState(false);
    const [progress, setProgress] = useState<DownloadProgress | null>(null);
    const [error, setError] = useState<string | null>(null);

    const loadStatus = async () => {
        setLoading(true);
        setError(null);
        try {
            const s = await invoke<ModelStatus>("get_model_status");
            setStatus(s);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadStatus();

        const unlistenProgress = listen<DownloadProgress>("model-download-progress", (event) => {
            setProgress(event.payload);
        });

        const unlistenComplete = listen("model-download-complete", () => {
            setDownloading(false);
            setProgress(null);
            loadStatus();
        });

        const unlistenUpdate = listen<string>("model-update-available", () => {
            loadStatus();
        });

        return () => {
            unlistenProgress.then((f) => f());
            unlistenComplete.then((f) => f());
            unlistenUpdate.then((f) => f());
        };
    }, []);

    const handleDownload = async () => {
        setDownloading(true);
        setError(null);
        setProgress(null);
        try {
            await invoke("download_embedding_model");
        } catch (e) {
            setError(String(e));
            setDownloading(false);
        }
    };

    const handleDelete = async () => {
        try {
            await invoke("delete_downloaded_model");
            loadStatus();
        } catch (e) {
            setError(String(e));
        }
    };

    const sourceLabel = (source: string) => {
        switch (source) {
            case "bundled": return t("searchModel.sourceBundled");
            case "downloaded": return t("searchModel.sourceDownloaded");
            default: return t("searchModel.sourceNone");
        }
    };

    return (
        <Stack gap="md">
            <Text fw={600} size="lg">{t("searchModel.title")}</Text>
            <Text size="sm" c="dimmed">{t("searchModel.description")}</Text>

            {error && (
                <Alert color="red" icon={<IconAlertTriangle size={16} />}>
                    {error}
                </Alert>
            )}

            {status && (
                <Paper p="md" withBorder>
                    <Stack gap="sm">
                        <Group justify="space-between">
                            <Group gap="xs">
                                <IconBrain size={18} stroke={1.5} />
                                <Text fw={500}>multilingual-e5-small</Text>
                            </Group>
                            <Badge
                                color={status.installed ? "green" : "red"}
                                variant="light"
                            >
                                {status.installed ? t("searchModel.installed") : t("searchModel.notInstalled")}
                            </Badge>
                        </Group>

                        <Group gap="lg">
                            <Text size="sm" c="dimmed">
                                {t("searchModel.source")}: {sourceLabel(status.source)}
                            </Text>
                            {status.current_version && (
                                <Text size="sm" c="dimmed">
                                    {t("searchModel.version")}: {status.current_version}
                                </Text>
                            )}
                            {status.latest_version && (
                                <Text size="sm" c="dimmed">
                                    {t("searchModel.latestVersion")}: {status.latest_version}
                                </Text>
                            )}
                            {status.model_size_mb && (
                                <Text size="sm" c="dimmed">
                                    {t("searchModel.size")}: {status.model_size_mb.toFixed(1)} MB
                                </Text>
                            )}
                        </Group>

                        {downloading && progress && (
                            <Stack gap="xs">
                                <Text size="sm">
                                    {t("searchModel.downloading")}: {progress.file_name} ({progress.percent.toFixed(1)}%)
                                </Text>
                                <Progress
                                    value={progress.percent}
                                    size="lg"
                                    animated
                                    color="brand"
                                />
                                <Text size="xs" c="dimmed">
                                    {(progress.downloaded / 1048576).toFixed(1)} / {(progress.total / 1048576).toFixed(1)} MB
                                </Text>
                            </Stack>
                        )}

                        <Group gap="sm">
                            {status.update_available && !downloading && (
                                <Button
                                    leftSection={status.installed ? <IconRefresh size={16} /> : <IconDownload size={16} />}
                                    onClick={handleDownload}
                                    loading={downloading}
                                    color="brand"
                                    size="sm"
                                >
                                    {status.source === "none"
                                        ? t("searchModel.download")
                                        : t("searchModel.update")}
                                </Button>
                            )}
                            {status.source === "downloaded" && !downloading && (
                                <Button
                                    leftSection={<IconTrash size={16} />}
                                    onClick={handleDelete}
                                    variant="subtle"
                                    color="red"
                                    size="sm"
                                >
                                    {t("searchModel.deleteDownloaded")}
                                </Button>
                            )}
                            <Button
                                leftSection={<IconRefresh size={16} />}
                                onClick={loadStatus}
                                variant="subtle"
                                size="sm"
                                loading={loading}
                            >
                                {t("searchModel.checkUpdate")}
                            </Button>
                        </Group>

                        {status.installed && status.source === "downloaded" && !downloading && (
                            <Alert color="yellow" icon={<IconAlertTriangle size={16} />}>
                                {t("searchModel.restartRequired")}
                            </Alert>
                        )}
                    </Stack>
                </Paper>
            )}
        </Stack>
    );
}
