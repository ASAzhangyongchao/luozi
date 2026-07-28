import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const tokenFile = resolve(process.cwd(), "src/design-tokens.css");
const approvedTokens = {
  "--surface-pearl": "#FBFDF9",
  "--surface-mist": "#EEF7F5",
  "--text-ink": "#173A3A",
  "--brand-mint": "#62D5C7",
  "--brand-violet": "#8B92FF",
  "--state-success": "#50C59F",
  "--state-error": "#F1786D",
};

describe("Luozi design tokens", () => {
  it("defines the approved semantic palette", () => {
    const css = readFileSync(tokenFile, "utf8");

    for (const [token, value] of Object.entries(approvedTokens)) {
      expect(css).toMatch(new RegExp(`${token}\\s*:\\s*${value}`, "i"));
    }
  });

  it("provides reduced-motion and reduced-transparency fallbacks", () => {
    const css = readFileSync(tokenFile, "utf8");

    expect(css).toMatch(/@media\s*\(prefers-reduced-motion:\s*reduce\)/i);
    expect(css).toMatch(/@media\s*\(prefers-reduced-transparency:\s*reduce\)/i);
  });
});
