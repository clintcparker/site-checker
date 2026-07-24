import { describe, expect, it } from "vitest";
import { formatSince } from "./time";

const NOW = 1_700_000_000_000;

describe("formatSince", () => {
  it("shows an em dash when there is no check yet", () => {
    expect(formatSince(null, NOW)).toBe("—");
  });

  it("shows seconds under a minute", () => {
    expect(formatSince(NOW, NOW)).toBe("0s ago");
    expect(formatSince(NOW - 5_000, NOW)).toBe("5s ago");
    expect(formatSince(NOW - 59_000, NOW)).toBe("59s ago");
  });

  it("shows whole minutes from one minute up", () => {
    expect(formatSince(NOW - 60_000, NOW)).toBe("1m ago");
    expect(formatSince(NOW - 119_000, NOW)).toBe("1m ago");
    expect(formatSince(NOW - 180_000, NOW)).toBe("3m ago");
    expect(formatSince(NOW - 59 * 60_000, NOW)).toBe("59m ago");
  });

  it("shows whole hours from one hour up", () => {
    expect(formatSince(NOW - 3_600_000, NOW)).toBe("1h ago");
    expect(formatSince(NOW - 7_200_000, NOW)).toBe("2h ago");
  });

  it("never shows a negative age when the clock jitters", () => {
    expect(formatSince(NOW + 500, NOW)).toBe("0s ago");
  });
});
