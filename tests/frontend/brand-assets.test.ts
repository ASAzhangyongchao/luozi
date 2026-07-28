import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const tray = readFileSync(
  resolve(process.cwd(), "assets/brand/luozi-tray-template.svg"),
  "utf8",
);

describe("tray template asset", () => {
  it("is a monochrome 16 px breathing bubble with one orbit", () => {
    expect(tray).toContain('viewBox="0 0 16 16"');
    expect(tray).toContain('class="bubble"');
    expect(tray).toContain('class="orbit"');
    expect(tray).not.toContain("<linearGradient");
    expect(tray).not.toMatch(/#[0-9a-f]{3,8}/i);
  });
});
