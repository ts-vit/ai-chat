import { describe, it, expect } from "vitest";
import { cronToHuman, CRON_PRESETS } from "../cronHelper";

describe("cronToHuman", () => {
  it("every N minutes (en)", () => {
    expect(cronToHuman("*/30 * * * *", "en")).toBe("Every 30 minutes");
  });

  it("every N minutes (ru)", () => {
    expect(cronToHuman("*/30 * * * *", "ru")).toBe("Каждые 30 минут");
  });

  it("every hour (en)", () => {
    expect(cronToHuman("0 * * * *", "en")).toBe("Every hour");
  });

  it("every hour (ru)", () => {
    expect(cronToHuman("0 * * * *", "ru")).toBe("Каждый час");
  });

  it("every N hours (en)", () => {
    expect(cronToHuman("0 */6 * * *", "en")).toBe("Every 6 hours");
  });

  it("daily at time (en)", () => {
    expect(cronToHuman("0 9 * * *", "en")).toBe("Every day at 09:00");
  });

  it("daily at time (ru)", () => {
    expect(cronToHuman("0 9 * * *", "ru")).toBe("Каждый день в 09:00");
  });

  it("specific weekday (en)", () => {
    expect(cronToHuman("0 10 * * 1", "en")).toBe("Every Monday at 10:00");
  });

  it("specific weekday (ru)", () => {
    expect(cronToHuman("0 18 * * 5", "ru")).toBe("Каждый(ую) пятницу в 18:00");
  });

  it("unrecognized pattern returns raw cron", () => {
    expect(cronToHuman("5 4 * * 1,3,5", "en")).toBe("5 4 * * 1,3,5");
  });

  it("wrong number of parts returns raw cron", () => {
    expect(cronToHuman("* *", "en")).toBe("* *");
  });
});

describe("CRON_PRESETS", () => {
  it("has 7 presets", () => {
    expect(CRON_PRESETS).toHaveLength(7);
  });

  it("each preset has labelEn, labelRu, cron", () => {
    for (const preset of CRON_PRESETS) {
      expect(preset.labelEn).toBeTruthy();
      expect(preset.labelRu).toBeTruthy();
      expect(preset.cron).toBeTruthy();
    }
  });
});
