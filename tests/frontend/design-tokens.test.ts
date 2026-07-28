import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const tokenFile = resolve(process.cwd(), "src/design-tokens.css");
const overlayFile = resolve(process.cwd(), "src/overlay.css");
const settingsFile = resolve(process.cwd(), "src/styles-settings.css");
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

    expect(css).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*:root\s*\{[\s\S]*?--motion-fast\s*:\s*0ms\s*;[\s\S]*?--motion-state\s*:\s*0ms\s*;/i,
    );
    expect(css).toMatch(
      /@media\s*\(prefers-reduced-transparency:\s*reduce\)\s*\{\s*:root\s*\{[\s\S]*?--surface-pearl\s*:\s*var\(--surface-solid\)\s*;/i,
    );
  });

  it("defines readable success foreground colors for light and dark surfaces", () => {
    const css = readFileSync(tokenFile, "utf8");

    expect(css).toMatch(/--state-success-foreground\s*:\s*#1a7f4b\s*;/i);
    expect(css).toMatch(
      /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{[\s\S]*?--state-success-foreground\s*:\s*#75d8b5\s*;/i,
    );
  });

  it("imports tokens first and keeps settings aliases semantic", () => {
    const overlayCss = readFileSync(overlayFile, "utf8");
    const settingsCss = readFileSync(settingsFile, "utf8");
    const settingsAliases = {
      "--bg": "--surface-mist",
      "--panel": "--surface-pearl",
      "--line": "--line-soft",
      "--text": "--text-ink",
      "--muted": "--text-muted",
      "--accent": "--brand-mint",
      "--ok": "--state-success",
    };

    expect(overlayCss.startsWith('@import "./design-tokens.css";')).toBe(true);
    expect(settingsCss.startsWith('@import "./design-tokens.css";')).toBe(true);

    for (const [alias, token] of Object.entries(settingsAliases)) {
      expect(settingsCss).toMatch(new RegExp(`${alias}\\s*:\\s*var\\(${token}\\)`, "i"));
    }
  });

  it("uses the readable success foreground for success badge text", () => {
    const settingsCss = readFileSync(settingsFile, "utf8");

    expect(settingsCss).toMatch(
      /\.badge\.ok\s*\{[^}]*color\s*:\s*var\(--state-success-foreground\)\s*;/i,
    );
  });
});
