import { describe, it, expect, vi } from "vitest";

vi.mock("@tabler/icons-react", () => ({
  IconMessage: () => null,
  IconRobot: () => null,
  IconNotebook: () => null,
}));

import { MODE_DEFINITIONS, DEFAULT_MODE } from "../modes";

describe("MODE_DEFINITIONS", () => {
  it("has exactly 3 modes", () => {
    expect(MODE_DEFINITIONS).toHaveLength(3);
  });

  it("contains chat, assistant and notebook modes", () => {
    const ids = MODE_DEFINITIONS.map((m) => m.id);
    expect(ids).toContain("chat");
    expect(ids).toContain("assistant");
    expect(ids).toContain("notebook");
  });

  it("all modes have required fields", () => {
    for (const mode of MODE_DEFINITIONS) {
      expect(mode.id).toBeTruthy();
      expect(mode.labelKey).toBeTruthy();
      expect(mode.descriptionKey).toBeTruthy();
      expect(mode.color).toBeTruthy();
      expect(typeof mode.defaultEnabled).toBe("boolean");
    }
  });

  it("mode IDs are unique", () => {
    const ids = MODE_DEFINITIONS.map((m) => m.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("DEFAULT_MODE", () => {
  it("is 'chat'", () => {
    expect(DEFAULT_MODE).toBe("chat");
  });
});
