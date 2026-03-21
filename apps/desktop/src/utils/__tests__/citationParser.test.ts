import { describe, test, expect } from "vitest";
import { extractCitationIndices } from "../citationParser";

describe("extractCitationIndices", () => {
    test("extracts single citation", () => {
        expect(extractCitationIndices("OAuth uses tokens [1].")).toEqual([1]);
    });

    test("extracts multiple citations", () => {
        expect(extractCitationIndices("Auth [1] and tokens [2] are important [3].")).toEqual([1, 2, 3]);
    });

    test("extracts unique sorted indices from duplicates", () => {
        expect(extractCitationIndices("See [2] and [1] and again [2].")).toEqual([1, 2]);
    });

    test("handles consecutive citations [1][2]", () => {
        expect(extractCitationIndices("Both support pagination [1][2].")).toEqual([1, 2]);
    });

    test("handles comma-separated [1, 3] as individual brackets", () => {
        // [1, 3] won't match \[\d{1,3}\] — only individual [N] will
        expect(extractCitationIndices("Sources [1] and [3] are used.")).toEqual([1, 3]);
    });

    test("empty text returns empty", () => {
        expect(extractCitationIndices("")).toEqual([]);
    });

    test("no citations returns empty", () => {
        expect(extractCitationIndices("No citations here.")).toEqual([]);
    });

    test("ignores [0] since indices are 1-based", () => {
        expect(extractCitationIndices("Array[0] and [1] are different.")).toEqual([1]);
    });

    test("handles large index numbers", () => {
        expect(extractCitationIndices("Source [12] and [100].")).toEqual([12, 100]);
    });

    test("does not match 4+ digit numbers", () => {
        expect(extractCitationIndices("Value [1234] is not a citation.")).toEqual([]);
    });
});
