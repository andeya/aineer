import { describe, expect, test } from "bun:test";
import { INTERACTIVE_COMMANDS, isInteractiveCommand } from "../lib/constants";

describe("INTERACTIVE_COMMANDS", () => {
  test("contains expected commands", () => {
    expect(INTERACTIVE_COMMANDS.has("vim")).toBe(true);
    expect(INTERACTIVE_COMMANDS.has("ssh")).toBe(true);
    expect(INTERACTIVE_COMMANDS.has("python3")).toBe(true);
    expect(INTERACTIVE_COMMANDS.has("mysql")).toBe(true);
  });

  test("does not contain non-interactive commands", () => {
    expect(INTERACTIVE_COMMANDS.has("ls")).toBe(false);
    expect(INTERACTIVE_COMMANDS.has("cat")).toBe(false);
    expect(INTERACTIVE_COMMANDS.has("echo")).toBe(false);
  });
});

describe("isInteractiveCommand", () => {
  test("detects interactive commands", () => {
    expect(isInteractiveCommand("vim")).toBe(true);
    expect(isInteractiveCommand("vim file.txt")).toBe(true);
    expect(isInteractiveCommand("ssh user@host")).toBe(true);
    expect(isInteractiveCommand("  python3  ")).toBe(true);
  });

  test("rejects non-interactive commands", () => {
    expect(isInteractiveCommand("ls -la")).toBe(false);
    expect(isInteractiveCommand("echo hello")).toBe(false);
    expect(isInteractiveCommand("cargo build")).toBe(false);
  });

  test("handles edge cases", () => {
    expect(isInteractiveCommand("")).toBe(false);
    expect(isInteractiveCommand("   ")).toBe(false);
  });
});
