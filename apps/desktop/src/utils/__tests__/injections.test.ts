import { describe, it, expect } from "vitest";
import { hasInjections } from "../injections";

describe("hasInjections", () => {
  it("detects {{date}}", () => {
    expect(hasInjections("Today is {{date}}")).toBe(true);
  });

  it("detects {{clipboard}}", () => {
    expect(hasInjections("Paste: {{clipboard}}")).toBe(true);
  });

  it("detects {{file:path}}", () => {
    expect(hasInjections("Content: {{file:D:/test.txt}}")).toBe(true);
  });

  it("detects {{date:FORMAT}}", () => {
    expect(hasInjections("{{date:YYYY-MM-DD}}")).toBe(true);
  });

  it("returns false for plain text", () => {
    expect(hasInjections("Hello world")).toBe(false);
  });

  it("returns false for empty string", () => {
    expect(hasInjections("")).toBe(false);
  });

  it("returns false for single braces", () => {
    expect(hasInjections("{single}")).toBe(false);
  });

  it("returns false for empty braces", () => {
    expect(hasInjections("{{}}")).toBe(false);
  });
});
