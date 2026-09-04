import { describe, expect, it, vi } from "vitest";
import { nextFocusIndex, persistWithRollback } from "./settingsBehavior";

describe("nextFocusIndex", () => {
  it("wraps focus forward and backward inside a modal", () => {
    expect(nextFocusIndex(3, 2, false)).toBe(0);
    expect(nextFocusIndex(3, 0, true)).toBe(2);
    expect(nextFocusIndex(3, 1, false)).toBe(2);
  });
});

describe("persistWithRollback", () => {
  it("rolls back and rethrows when persistence fails", async () => {
    const rollback = vi.fn();
    const error = new Error("disk unavailable");

    await expect(persistWithRollback(() => Promise.reject(error), rollback)).rejects.toBe(error);
    expect(rollback).toHaveBeenCalledOnce();
  });

  it("does not roll back a successful persistence", async () => {
    const rollback = vi.fn();
    await persistWithRollback(() => Promise.resolve(), rollback);
    expect(rollback).not.toHaveBeenCalled();
  });
});
