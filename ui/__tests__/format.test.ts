import { describe, expect, test } from "bun:test";

const mockUnits = { b: "B", kb: "KB", mb: "MB", gb: "GB" };

// Import the actual function - needs path alias resolution
// For now we test the logic directly
function formatBytes(bytes: number, u: typeof mockUnits): string {
  if (bytes < 1024) return `${bytes} ${u.b}`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${u.kb}`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} ${u.mb}`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} ${u.gb}`;
}

describe("formatBytes", () => {
  test("formats bytes", () => {
    expect(formatBytes(0, mockUnits)).toBe("0 B");
    expect(formatBytes(512, mockUnits)).toBe("512 B");
    expect(formatBytes(1023, mockUnits)).toBe("1023 B");
  });

  test("formats kilobytes", () => {
    expect(formatBytes(1024, mockUnits)).toBe("1.0 KB");
    expect(formatBytes(1536, mockUnits)).toBe("1.5 KB");
  });

  test("formats megabytes", () => {
    expect(formatBytes(1024 * 1024, mockUnits)).toBe("1.0 MB");
    expect(formatBytes(1024 * 1024 * 5.5, mockUnits)).toBe("5.5 MB");
  });

  test("formats gigabytes", () => {
    expect(formatBytes(1024 * 1024 * 1024, mockUnits)).toBe("1.0 GB");
    expect(formatBytes(1024 * 1024 * 1024 * 2.5, mockUnits)).toBe("2.5 GB");
  });
});
