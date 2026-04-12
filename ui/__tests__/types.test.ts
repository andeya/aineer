import { describe, expect, test } from "bun:test";
import type { ChatMessage, InputMode, ToolPart } from "../lib/types";

describe("Type definitions", () => {
  test("InputMode accepts valid values", () => {
    const modes: InputMode[] = ["shell", "ai", "agent"];
    expect(modes).toHaveLength(3);
  });

  test("ChatMessage shape", () => {
    const msg: ChatMessage = {
      id: 1,
      role: "user",
      mode: "ai",
      content: "Hello",
      timestamp: Date.now(),
    };
    expect(msg.role).toBe("user");
    expect(msg.mode).toBe("ai");
  });

  test("ToolPart shape", () => {
    const tool: ToolPart = {
      type: "search_files",
      state: "output-available",
      input: { query: "test" },
      output: { matches: 3 },
    };
    expect(tool.type).toBe("search_files");
    expect(tool.state).toBe("output-available");
  });
});
