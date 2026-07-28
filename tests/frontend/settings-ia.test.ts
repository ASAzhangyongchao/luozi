import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const html = readFileSync(resolve(process.cwd(), "settings.html"), "utf8");
const source = readFileSync(resolve(process.cwd(), "src/settings.ts"), "utf8");
const css = readFileSync(resolve(process.cwd(), "src/styles-settings.css"), "utf8");

function section(id: string, nextId?: string) {
  const start = html.indexOf(`id="section-${id}"`);
  const end = nextId ? html.indexOf(`id="section-${nextId}"`, start) : html.length;
  return html.slice(start, end);
}

describe("settings information architecture", () => {
  it("uses the approved six navigation groups with About anchored last", () => {
    const sections = [
      ...html.matchAll(/class="nav-item[^"]*" data-section="([^"]+)"/g),
    ].map((match) => match[1]);

    expect(sections).toEqual([
      "general",
      "voice",
      "hotkeys",
      "privacy",
      "advanced",
      "about",
    ]);
    expect(html).toMatch(
      /class="nav-item nav-about" data-section="about"[^>]*>关于<\/button>/,
    );
  });

  it("keeps provider, key and consent controls inside Advanced", () => {
    const voice = section("voice", "hotkeys");
    const advanced = section("advanced", "about");
    const technicalControls = [
      "btnPickAsrProvider",
      "btnAsrKey",
      "btnAsrConsent",
      "btnPickTextProvider",
      "btnTextAiKey",
      "btnTextAiModel",
      "btnTextAiConsent",
    ];

    for (const id of technicalControls) {
      expect(voice).not.toContain(`id="${id}"`);
      expect(advanced).toContain(`id="${id}"`);
    }
    expect(voice).toContain('id="btnPickAsrMode"');
    expect(voice).toContain('id="modelStatus"');
    expect(voice).toContain('id="textAiSummary"');
    expect(voice).toContain('id="textAiSummaryBadge"');
  });

  it("shows only truthful persisted behavior in General", () => {
    const general = section("general", "voice");

    expect(general).toContain("关闭窗口后继续运行");
    expect(general).toContain("当前草稿自动恢复");
    expect(general).toContain("界面语言");
    expect(general).not.toContain("登录时启动");
    expect(general).not.toContain("暂不可用");
    expect(general).not.toMatch(/type="checkbox"|role="switch"/);
  });

  it("polls permissions only while Privacy is active", () => {
    expect(source).toMatch(/const isPrivacy = id === "privacy";/);
    expect(source).toMatch(/syncPermissionsPoll\(isPrivacy\);/);
    expect(source).toMatch(/if \(isPrivacy\) void refresh\(\);/);
    expect(source).not.toContain('id === "permissions"');
  });

  it("summarizes text AI readiness without exposing technical controls", () => {
    expect(source).toContain('document.querySelector("#textAiSummary")');
    expect(source).toContain('document.querySelector("#textAiSummaryBadge")');
    expect(source).toContain("AI 修改已就绪");
    expect(source).toContain("AI 已就绪");
  });

  it("preserves all three local model readiness states", () => {
    expect(source).toContain('s.modelStatus.includes("需重新下载")');
    expect(source).toContain('"需重下"');
    expect(source).toContain('"未安装"');
    expect(source).toContain('"已就绪"');
  });

  it("uses the approved compact tokenized settings layout", () => {
    expect(css).toMatch(
      /\.settings-shell\s*\{[^}]*grid-template-columns:\s*156px minmax\(0,\s*1fr\)/s,
    );
    expect(css).toMatch(/\.settings-nav\s*\{[^}]*gap:\s*3px[^}]*padding:\s*14px 10px/s);
    expect(css).toMatch(/\.nav-item\s*\{[^}]*min-height:\s*34px/s);
    expect(css).toMatch(/\.settings-main\s*\{[^}]*padding:\s*28px 32px 40px/s);
    expect(css).toMatch(/\.settings-group \.row\s*\{[^}]*min-height:\s*58px/s);
    expect(css).not.toContain("radial-gradient");
    expect(css).toContain("var(--surface-mist)");
    expect(css).toContain("var(--surface-pearl)");
  });
});
