import { describe, it, expect } from "vitest";
import { formatTokenCount } from "../tokenCount";

describe("formatTokenCount", () => {
  it("below 1000 returns raw number", () => {
    expect(formatTokenCount(42)).toBe("42");
  });

  it("zero returns '0'", () => {
    expect(formatTokenCount(0)).toBe("0");
  });

  it("at 1000 returns '1.0k'", () => {
    expect(formatTokenCount(1000)).toBe("1.0k");
  });

  it("between 1k and 10k returns one decimal", () => {
    expect(formatTokenCount(1234)).toBe("1.2k");
    expect(formatTokenCount(5678)).toBe("5.7k");
  });

  it("at 10000 returns '10k'", () => {
    expect(formatTokenCount(10000)).toBe("10k");
  });

  it("large number returns rounded k", () => {
    expect(formatTokenCount(150000)).toBe("150k");
  });

  it("999 returns '999'", () => {
    expect(formatTokenCount(999)).toBe("999");
  });
});
