import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  Button,
  Checkbox,
  Group,
  Loader,
  Pagination,
  SegmentedControl,
  Table,
  Text,
  TextInput,
  Tooltip,
} from "@mantine/core";
import { IconSearch } from "@tabler/icons-react";
import type { OpenRouterModel } from "./types";

const POPULAR_PREFIXES = [
  "anthropic/",
  "openai/",
  "google/",
  "meta-llama/",
  "deepseek/",
  "mistralai/",
];
const PAGE_SIZE = 20;

type FilterSegment = "all" | "popular" | "free" | "selected";
type SortKey = "name" | "prompt" | "completion" | "context";

function formatContextLength(n: number): string {
  if (n >= 1_000_000) return `${Math.round(n / 1_000_000)}M`;
  if (n >= 1000) return `${Math.round(n / 1000)}K`;
  return String(n);
}

function formatPrice(priceStr: string): string {
  const perM = parseFloat(priceStr) * 1_000_000;
  return `$${perM.toFixed(2)}`;
}

export interface ModelCatalogProps {
  models: OpenRouterModel[];
  loading: boolean;
  error: string | null;
  onReload: () => void;
  selectedIds: string[];
  onToggleModel: (id: string) => void;
}

export function ModelCatalog({
  models,
  loading,
  error,
  onReload,
  selectedIds,
  onToggleModel,
}: ModelCatalogProps) {
  const { t } = useTranslation();

  const [filterSegment, setFilterSegment] = useState<FilterSegment>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [page, setPage] = useState(1);

  const filteredModels = useMemo(() => {
    let list = models;

    if (filterSegment === "popular") {
      list = list.filter((m) =>
        POPULAR_PREFIXES.some((p) => m.id.startsWith(p))
      );
    } else if (filterSegment === "free") {
      list = list.filter(
        (m) =>
          parseFloat(m.pricing.prompt) === 0 &&
          parseFloat(m.pricing.completion) === 0
      );
    } else if (filterSegment === "selected") {
      list = list.filter((m) => selectedIds.includes(m.id));
    }

    const q = searchQuery.trim().toLowerCase();
    if (q) {
      list = list.filter(
        (m) =>
          m.name.toLowerCase().includes(q) ||
          m.id.toLowerCase().includes(q)
      );
    }

    if (sortKey) {
      list = [...list].sort((a, b) => {
        let va: string | number;
        let vb: string | number;
        switch (sortKey) {
          case "name":
            va = a.name.toLowerCase();
            vb = b.name.toLowerCase();
            break;
          case "prompt":
            va = parseFloat(a.pricing.prompt);
            vb = parseFloat(b.pricing.prompt);
            break;
          case "completion":
            va = parseFloat(a.pricing.completion);
            vb = parseFloat(b.pricing.completion);
            break;
          case "context":
            va = a.context_length;
            vb = b.context_length;
            break;
          default:
            return 0;
        }
        const cmp = va < vb ? -1 : va > vb ? 1 : 0;
        return sortDir === "asc" ? cmp : -cmp;
      });
    }

    return list;
  }, [models, filterSegment, searchQuery, sortKey, sortDir, selectedIds]);

  const totalPages = Math.max(1, Math.ceil(filteredModels.length / PAGE_SIZE));
  const pageModels = useMemo(() => {
    const start = (page - 1) * PAGE_SIZE;
    return filteredModels.slice(start, start + PAGE_SIZE);
  }, [filteredModels, page]);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
    setPage(1);
  };

  const sortArrow = (key: SortKey) => {
    if (sortKey !== key) return null;
    return sortDir === "asc" ? " \u2191" : " \u2193";
  };

  return (
    <>
      <Group justify="space-between" wrap="nowrap">
        <Text size="sm" fw={500}>
          {t("settings.openrouter.modelsTitle")}
        </Text>
        <Text size="xs" c="dimmed">
          {t("settings.openrouter.selectedCount", { count: selectedIds.length })}
        </Text>
      </Group>
      <Group wrap="nowrap" align="flex-end" gap="sm">
        <SegmentedControl
          value={filterSegment}
          onChange={(v) => { setFilterSegment(v as FilterSegment); setPage(1); }}
          data={[
            { label: t("settings.openrouter.all"), value: "all" },
            { label: t("settings.openrouter.popular"), value: "popular" },
            { label: t("settings.openrouter.free"), value: "free" },
            { label: t("settings.openrouter.selected"), value: "selected" },
          ]}
        />
        <TextInput
          placeholder={t("settings.openrouter.searchPlaceholder")}
          leftSection={<IconSearch size={16} stroke={1.5} />}
          value={searchQuery}
          onChange={(e) => { setSearchQuery(e.currentTarget.value); setPage(1); }}
          style={{ flex: 1, minWidth: 200 }}
        />
      </Group>

      {error && (
        <Group gap="sm">
          <Text size="sm" c="red">{error}</Text>
          <Button variant="light" size="xs" onClick={onReload}>
            {t("settings.openrouter.retry")}
          </Button>
        </Group>
      )}

      {loading && (
        <Group justify="center" py="xl">
          <Loader />
        </Group>
      )}

      {!loading && !error &&
        filterSegment === "selected" &&
        filteredModels.length === 0 && (
          <Text size="sm" c="dimmed" py="md">
            {t("settings.openrouter.noSelectedModels")}
          </Text>
        )}

      {!loading &&
        !error &&
        !(filterSegment === "selected" && filteredModels.length === 0) && (
          <Table
            withTableBorder
            withColumnBorders
            striped
            highlightOnHover
          >
            <Table.Thead>
              <Table.Tr>
                <Table.Th style={{ width: 48 }} />
                <Table.Th
                  style={{ cursor: "pointer", width: "auto" }}
                  onClick={() => handleSort("name")}
                >
                  {t("settings.openrouter.model")}{sortArrow("name")}
                </Table.Th>
                <Table.Th
                  style={{ cursor: "pointer", width: 110 }}
                  onClick={() => handleSort("prompt")}
                >
                  {t("settings.openrouter.input")}{sortArrow("prompt")}
                </Table.Th>
                <Table.Th
                  style={{ cursor: "pointer", width: 110 }}
                  onClick={() => handleSort("completion")}
                >
                  {t("settings.openrouter.output")}{sortArrow("completion")}
                </Table.Th>
                <Table.Th
                  style={{ cursor: "pointer", width: 110 }}
                  onClick={() => handleSort("context")}
                >
                  {t("settings.openrouter.context")}{sortArrow("context")}
                </Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {pageModels.map((m) => {
                const checked = selectedIds.includes(m.id);
                return (
                  <Table.Tr
                    key={m.id}
                    style={{ cursor: "pointer" }}
                    onClick={() => onToggleModel(m.id)}
                  >
                    <Table.Td style={{ width: 48, verticalAlign: "middle" }}>
                      <Checkbox
                        checked={checked}
                        onChange={() => onToggleModel(m.id)}
                        onClick={(e) => e.stopPropagation()}
                        aria-label={m.name}
                      />
                    </Table.Td>
                    <Table.Td>
                      {m.description?.trim() ? (
                        <Tooltip
                          label={m.description}
                          multiline
                          maw={400}
                        >
                          <Box>
                            <Text size="sm">{m.name}</Text>
                            <Text size="xs" c="dimmed">{m.id}</Text>
                          </Box>
                        </Tooltip>
                      ) : (
                        <Box>
                          <Text size="sm">{m.name}</Text>
                          <Text size="xs" c="dimmed">{m.id}</Text>
                        </Box>
                      )}
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{formatPrice(m.pricing.prompt)}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{formatPrice(m.pricing.completion)}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{formatContextLength(m.context_length)}</Text>
                    </Table.Td>
                  </Table.Tr>
                );
              })}
            </Table.Tbody>
          </Table>
        )}

      {!loading &&
        !error &&
        !(filterSegment === "selected" && filteredModels.length === 0) &&
        totalPages > 1 && (
          <Group justify="center" mt="sm">
            <Pagination
              total={totalPages}
              value={page}
              onChange={setPage}
            />
          </Group>
        )}
    </>
  );
}
