import { describe, expect, it } from "vitest";
import { endpointHost, splitAssistantResponse } from "./response";

describe("splitAssistantResponse", () => {
  it("leaves ordinary prose untouched", () => {
    expect(splitAssistantResponse("Hello")).toEqual({ message: "Hello", hasActionBlock: false });
  });

  it("removes a fenced action block from visible prose", () => {
    const value = "Updated.\n```foldown-actions\n{\"actions\":[]}\n```";
    expect(splitAssistantResponse(value)).toEqual({ message: "Updated.", hasActionBlock: true });
  });
  it("hides generic and partial JSON action blocks", () => {
    expect(splitAssistantResponse("Done.\n```json\n[{\"action\":\"replace\"}]").message).toBe("Done.");
  });

  it("keeps unrelated JSON examples visible", () => {
    const value = "Example:\n```json\n{\"approved\":true}\n```";
    expect(splitAssistantResponse(value)).toEqual({ message: value, hasActionBlock: false });
  });
});

describe("endpointHost", () => {
  it("shows the configured destination host", () => {
    expect(endpointHost("http://localhost:11434/v1")).toBe("localhost:11434");
  });
  it("does not throw for invalid input", () => {
    expect(endpointHost("bad url")).toBe("Invalid endpoint");
  });
});
