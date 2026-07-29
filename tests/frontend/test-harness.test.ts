import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("Luozi frontend test harness", () => {
  it("runs from the repository root", () => {
    const pkg = JSON.parse(readFileSync(resolve("package.json"), "utf8")) as {
      name: string;
    };
    expect(pkg.name).toBe("luozi");
  });
});
