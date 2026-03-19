import { describe, it, expect } from "vitest";
import {
  resolveProvider,
  extractTextContent,
  stripMarkdownForTts,
  parseRagData,
  parseWebSources,
  dbMessageToMessage,
  toMessageWithSiblings,
} from "../helpers";
import type { AppSettings, CustomProvider } from "../../types";

const baseSettings = {
  api_key: "test-key-123",
  ollamaUrl: "",
} as AppSettings;

describe("resolveProvider", () => {
  it("openrouter returns OpenRouter URL and api key", () => {
    const result = resolveProvider("openrouter", baseSettings, []);
    expect(result).toEqual({
      baseUrl: "https://openrouter.ai/api/v1",
      apiKey: "test-key-123",
    });
  });

  it("ollama returns default localhost URL", () => {
    const result = resolveProvider("ollama", baseSettings, []);
    expect(result).toEqual({
      baseUrl: "http://localhost:11434/v1",
      apiKey: "",
    });
  });

  it("ollama uses custom URL when set", () => {
    const settings = { ...baseSettings, ollamaUrl: "http://myserver:11434" };
    const result = resolveProvider("ollama", settings, []);
    expect(result!.baseUrl).toBe("http://myserver:11434");
  });

  it("custom provider found by id", () => {
    const providers: CustomProvider[] = [
      { id: "custom-1", name: "My Provider", baseUrl: "https://api.example.com/v1", apiKey: "ckey" } as CustomProvider,
    ];
    const result = resolveProvider("custom-1", baseSettings, providers);
    expect(result).toEqual({
      baseUrl: "https://api.example.com/v1",
      apiKey: "ckey",
    });
  });

  it("unknown provider returns null", () => {
    expect(resolveProvider("nonexistent", baseSettings, [])).toBeNull();
  });
});

describe("extractTextContent", () => {
  it("plain text returned as-is", () => {
    expect(extractTextContent("Hello world")).toBe("Hello world");
  });

  it("JSON ContentBlock array extracts text parts", () => {
    const json = JSON.stringify([
      { type: "text", text: "Hello" },
      { type: "image_url", image_url: { url: "http://img.png" } },
      { type: "text", text: "World" },
    ]);
    expect(extractTextContent(json)).toBe("Hello\nWorld");
  });

  it("invalid JSON returns original string", () => {
    expect(extractTextContent("[not json")).toBe("[not json");
  });

  it("non-array JSON returns original string", () => {
    const json = JSON.stringify({ type: "text", text: "Hello" });
    expect(extractTextContent(json)).toBe(json);
  });
});

describe("stripMarkdownForTts", () => {
  it("removes code blocks", () => {
    const input = "Before ```const x = 1;``` After";
    const result = stripMarkdownForTts(input);
    expect(result).toBe("Before After");
  });

  it("removes inline code", () => {
    expect(stripMarkdownForTts("Use `console.log` here")).toBe("Use here");
  });

  it("strips bold and italic", () => {
    expect(stripMarkdownForTts("**bold** and _italic_")).toBe("bold and italic");
  });

  it("strips headers", () => {
    expect(stripMarkdownForTts("# Title\nBody")).toBe("Title Body");
  });

  it("strips links, keeps text", () => {
    expect(stripMarkdownForTts("[click here](http://example.com)")).toBe("click here");
  });
});

describe("parseRagData", () => {
  it("old format: valid JSON array returns sources only", () => {
    const json = '[{"documentName":"test.md","content":"hello"}]';
    const result = parseRagData(json);
    expect(result?.sources).toEqual([{ documentName: "test.md", content: "hello" }]);
    expect(result?.trace).toBeUndefined();
  });

  it("new format: envelope with sources and trace", () => {
    const json = JSON.stringify({
      sources: [{ index: 1, documentName: "test.md", score: 0.9 }],
      trace: {
        originalQuery: "test query",
        variantsUsed: ["test query", "alt query"],
        subQuestions: [],
        totalCandidates: 20,
        afterDedup: 15,
        rerankerUsed: "cohere",
        rerankCandidates: 15,
        finalCount: 5,
      },
    });
    const result = parseRagData(json);
    expect(result?.sources).toHaveLength(1);
    expect(result?.trace?.originalQuery).toBe("test query");
    expect(result?.trace?.variantsUsed).toHaveLength(2);
    expect(result?.trace?.totalCandidates).toBe(20);
  });

  it("new format: envelope with optimization trace", () => {
    const json = JSON.stringify({
      sources: [{ index: 1, documentName: "doc.md", score: 0.8 }],
      trace: {
        originalQuery: "q",
        variantsUsed: ["q"],
        subQuestions: [],
        totalCandidates: 10,
        afterDedup: 8,
        rerankerUsed: null,
        rerankCandidates: 0,
        finalCount: 5,
        optimization: {
          originalTokens: 3000,
          afterSentenceExtraction: 2000,
          afterRedundancyRemoval: 1800,
          finalTokens: 1800,
          chunksBefore: 5,
          chunksAfter: 4,
        },
      },
    });
    const result = parseRagData(json);
    expect(result?.trace?.optimization?.originalTokens).toBe(3000);
    expect(result?.trace?.optimization?.chunksAfter).toBe(4);
  });

  it("null returns undefined", () => {
    expect(parseRagData(null)).toBeUndefined();
  });

  it("empty string returns undefined", () => {
    expect(parseRagData("")).toBeUndefined();
  });

  it("invalid JSON returns undefined", () => {
    expect(parseRagData("not json")).toBeUndefined();
  });

  it("empty array returns undefined", () => {
    expect(parseRagData("[]")).toBeUndefined();
  });

  it("envelope with empty sources returns undefined", () => {
    expect(parseRagData(JSON.stringify({ sources: [], trace: {} }))).toBeUndefined();
  });
});

describe("parseWebSources", () => {
  it("valid JSON array returns parsed sources", () => {
    const json = '[{"title":"Test","url":"http://example.com"}]';
    const result = parseWebSources(json);
    expect(result).toEqual([{ title: "Test", url: "http://example.com" }]);
  });

  it("null returns undefined", () => {
    expect(parseWebSources(null)).toBeUndefined();
  });
});

describe("dbMessageToMessage", () => {
  it("maps all fields correctly", () => {
    const dbMsg = {
      id: "msg-1",
      chatId: "chat-1",
      role: "assistant",
      content: "Hello",
      parentId: "msg-0",
      timestamp: 1700000000,
      model: "gpt-4",
      promptTokens: 100,
      completionTokens: 50,
      cost: 0.005,
      has_attachments: 0,
      webSources: undefined,
      ragSources: undefined,
      agentStep: undefined,
      agentRunId: undefined,
    };
    const msg = dbMessageToMessage(dbMsg);
    expect(msg.id).toBe("msg-1");
    expect(msg.role).toBe("assistant");
    expect(msg.content).toBe("Hello");
    expect(msg.parentId).toBe("msg-0");
    expect(msg.timestamp).toBe(1700000000);
    expect(msg.model).toBe("gpt-4");
    expect(msg.promptTokens).toBe(100);
    expect(msg.completionTokens).toBe(50);
    expect(msg.cost).toBe(0.005);
    expect(msg.hasAttachments).toBeUndefined();
  });

  it("has_attachments truthy maps to hasAttachments true", () => {
    const dbMsg = {
      id: "msg-2",
      chatId: "chat-1",
      role: "user",
      content: "With file",
      timestamp: 1700000000,
      has_attachments: 1,
    };
    const msg = dbMessageToMessage(dbMsg);
    expect(msg.hasAttachments).toBe(true);
  });

  it("parses old-format ragSources JSON (plain array)", () => {
    const dbMsg = {
      id: "msg-3",
      chatId: "chat-1",
      role: "assistant",
      content: "Answer",
      timestamp: 1700000000,
      ragSources: '[{"documentName":"test.md"}]',
    };
    const msg = dbMessageToMessage(dbMsg);
    expect(msg.ragSources).toBeDefined();
    expect(Array.isArray(msg.ragSources)).toBe(true);
    expect(msg.ragTrace).toBeUndefined();
  });

  it("parses new-format ragSources JSON (envelope with trace)", () => {
    const dbMsg = {
      id: "msg-4",
      chatId: "chat-1",
      role: "assistant",
      content: "Answer",
      timestamp: 1700000000,
      ragSources: JSON.stringify({
        sources: [{ index: 1, documentName: "test.md", score: 0.9 }],
        trace: { originalQuery: "test", variantsUsed: ["test"], subQuestions: [], totalCandidates: 10, afterDedup: 8, rerankerUsed: null, rerankCandidates: 0, finalCount: 5 },
      }),
    };
    const msg = dbMessageToMessage(dbMsg);
    expect(msg.ragSources).toHaveLength(1);
    expect(msg.ragTrace).toBeDefined();
    expect(msg.ragTrace?.originalQuery).toBe("test");
  });
});

describe("toMessageWithSiblings", () => {
  it("wraps message with sibling metadata", () => {
    const msg = {
      id: "msg-1",
      role: "user" as const,
      content: "Hello",
      timestamp: 1700000000,
    };
    const result = toMessageWithSiblings(msg);
    expect(result.siblingCount).toBe(1);
    expect(result.siblingIndex).toBe(0);
    expect(result.siblingIds).toEqual(["msg-1"]);
    expect(result.siblings).toEqual([]);
    expect(result.id).toBe("msg-1");
    expect(result.content).toBe("Hello");
  });
});
