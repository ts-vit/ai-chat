import { describe, it, expect } from "vitest";
import { tryParseContentBlocks } from "../contentParsing";

describe("tryParseContentBlocks", () => {
  it("null returns null", () => {
    expect(tryParseContentBlocks(null)).toBeNull();
  });

  it("undefined returns null", () => {
    expect(tryParseContentBlocks(undefined)).toBeNull();
  });

  it("empty string returns null", () => {
    expect(tryParseContentBlocks("")).toBeNull();
  });

  it("plain text returns null", () => {
    expect(tryParseContentBlocks("Hello world")).toBeNull();
  });

  it("valid ContentBlock array is parsed", () => {
    const json = JSON.stringify([{ type: "text", text: "Hello" }]);
    const result = tryParseContentBlocks(json);
    expect(result).toEqual([{ type: "text", text: "Hello" }]);
  });

  it("multiple blocks parsed correctly", () => {
    const json = JSON.stringify([
      { type: "text", text: "Hello" },
      { type: "image_url", image_url: { url: "http://example.com/img.png" } },
    ]);
    const result = tryParseContentBlocks(json);
    expect(result).toHaveLength(2);
    expect(result![0].type).toBe("text");
    expect(result![1].type).toBe("image_url");
  });

  it("non-array JSON returns null", () => {
    expect(tryParseContentBlocks('{"type":"text"}')).toBeNull();
  });

  it("array without type field returns null", () => {
    expect(tryParseContentBlocks('[{"noType": true}]')).toBeNull();
  });

  it("malformed JSON returns null", () => {
    expect(tryParseContentBlocks("[{invalid json")).toBeNull();
  });

  it("leading whitespace before array is handled", () => {
    const json = "  " + JSON.stringify([{ type: "text", text: "hi" }]);
    const result = tryParseContentBlocks(json);
    expect(result).toEqual([{ type: "text", text: "hi" }]);
  });
});
