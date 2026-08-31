import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";
import { installShim, type ShimSeed } from "./shim";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

/** 仓库真实样例导入文件内容（数据真实，不经手写 mock） */
export const importSample = readFileSync(
  `${repoRoot}dev-workflow-prompts.import.json`,
  "utf8",
);
export const sampleFirstTitle = (JSON.parse(importSample) as { prompts: { title: string }[] })
  .prompts[0].title;

const seedPrompts = [
  {
    id: "p1",
    title: "代码审查",
    content: "请审查以下代码的正确性与可读性：{{代码|贴入代码}}",
    category: "开发",
    tags: ["审查"],
    pinned: false,
    hotkey: "",
    useCount: 3,
    lastUsedAt: 1_700_000_000_000,
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_010_000,
  },
  {
    id: "p2",
    title: "周报生成",
    content: "本周完成：{{本周工作|本周完成的工作}}，下周计划：{{下周计划|下周要做的事}}",
    category: "写作",
    tags: [],
    pinned: false,
    hotkey: "",
    useCount: 1,
    lastUsedAt: 1_700_000_000_000,
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_005_000,
  },
  {
    id: "p3",
    title: "翻译助手",
    content: "把下面的内容翻译成英文，保持原意：",
    category: "写作",
    tags: [],
    pinned: false,
    hotkey: "",
    useCount: 0,
    lastUsedAt: 0,
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_001_000,
  },
];

export function baseSeed(label: string): ShimSeed {
  return {
    label,
    prompts: JSON.parse(JSON.stringify(seedPrompts)),
    categories: ["开发", "写作"],
    importSample,
    fresh: true,
  };
}

export async function openApp(page: Page, label: string) {
  await installShim(page, baseSeed(label));
  await page.goto("/");
  await page.waitForSelector(".mg-main, .qp");
}
