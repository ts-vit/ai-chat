import { describe, it, expect, vi } from "vitest";

vi.mock("../../../src/i18n", () => ({
  default: {
    t: (key: string, opts?: Record<string, unknown>) => {
      if (key === "dates.justNow") return "just now";
      if (key === "dates.minutesAgo") return `${opts?.count} minutes ago`;
      if (key === "dates.hoursAgo") return `${opts?.count} hours ago`;
      if (key === "dates.today") return "today";
      if (key === "dates.yesterday") return "yesterday";
      return key;
    },
    language: "en",
  },
}));

import { toUnixSeconds, formatRelativeTime, formatRelativeDate } from "../formatDate";

describe("toUnixSeconds", () => {
  it("passes through seconds", () => {
    expect(toUnixSeconds(1700000000)).toBe(1700000000);
  });

  it("converts milliseconds to seconds", () => {
    expect(toUnixSeconds(1700000000000)).toBe(1700000000);
  });

  it("boundary: at threshold (10_000_000_000) stays as seconds", () => {
    expect(toUnixSeconds(10_000_000_000)).toBe(10_000_000_000);
  });

  it("boundary: just above threshold converts", () => {
    expect(toUnixSeconds(10_000_000_001)).toBe(10_000_000);
  });

  it("handles zero", () => {
    expect(toUnixSeconds(0)).toBe(0);
  });
});

describe("formatRelativeTime", () => {
  it("returns 'just now' for < 60 seconds ago", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    expect(formatRelativeTime(nowSec - 30)).toBe("just now");
  });

  it("returns minutes ago", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    expect(formatRelativeTime(nowSec - 300)).toBe("5 minutes ago");
  });

  it("returns hours ago", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    expect(formatRelativeTime(nowSec - 7200)).toBe("2 hours ago");
  });
});

describe("formatRelativeDate", () => {
  it("returns 'today' for today's timestamp", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    expect(formatRelativeDate(nowSec)).toBe("today");
  });

  it("returns 'yesterday' for yesterday's timestamp", () => {
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    yesterday.setHours(12, 0, 0, 0);
    const sec = Math.floor(yesterday.getTime() / 1000);
    expect(formatRelativeDate(sec)).toBe("yesterday");
  });
});
