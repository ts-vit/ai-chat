import { describe, it, expect } from "vitest";
import { groupDocResults } from "../../components/SearchPage";
import type { KbDocSearchResult } from "../../types";

function makeChunk(overrides: Partial<KbDocSearchResult> = {}): KbDocSearchResult {
    return {
        chunkId: "c1",
        kbId: "kb1",
        kbName: "KB One",
        documentId: "d1",
        documentName: "doc.pdf",
        content: "some text",
        chunkIndex: 0,
        score: 0.9,
        ...overrides,
    };
}

describe("groupDocResults", () => {
    it("returns empty array for empty input", () => {
        expect(groupDocResults([])).toEqual([]);
    });

    it("groups single KB single doc", () => {
        const results = [makeChunk({ chunkId: "c1" }), makeChunk({ chunkId: "c2", chunkIndex: 1 })];
        const groups = groupDocResults(results);
        expect(groups).toHaveLength(1);
        expect(groups[0].kbId).toBe("kb1");
        expect(groups[0].kbName).toBe("KB One");
        expect(groups[0].documents).toHaveLength(1);
        expect(groups[0].documents[0].documentId).toBe("d1");
        expect(groups[0].documents[0].chunks).toHaveLength(2);
    });

    it("groups multiple KBs and documents", () => {
        const results = [
            makeChunk({ chunkId: "c1", kbId: "kb1", kbName: "KB1", documentId: "d1", documentName: "a.txt" }),
            makeChunk({ chunkId: "c2", kbId: "kb1", kbName: "KB1", documentId: "d2", documentName: "b.txt" }),
            makeChunk({ chunkId: "c3", kbId: "kb2", kbName: "KB2", documentId: "d3", documentName: "c.txt" }),
        ];
        const groups = groupDocResults(results);
        expect(groups).toHaveLength(2);
        expect(groups[0].kbId).toBe("kb1");
        expect(groups[0].documents).toHaveLength(2);
        expect(groups[1].kbId).toBe("kb2");
        expect(groups[1].documents).toHaveLength(1);
    });

    it("handles missing kbName gracefully", () => {
        const results = [makeChunk({ kbName: "" })];
        const groups = groupDocResults(results);
        expect(groups[0].kbName).toBe("");
    });
});
