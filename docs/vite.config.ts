import { defineConfig } from "vite";
import { defineTheme, oxContent } from "@ox-content/vite-plugin";
import { terminal } from "@ox-content/theme-terminal";
import { asciinema } from "./components/Asciinema.tsx";

const base = "/competitive-programming-cli/";

export default defineConfig({
  base,
  plugins: [
    oxContent({
      srcDir: "content",
      outDir: "dist",
      base,
      docs: false,
      highlight: true,
      mdx: true,
      transformers: [asciinema],
      fileTree: true,
      i18n: {
        enabled: true,
        check: false,
        defaultLocale: "en",
        locales: [
          { code: "en", name: "English" },
          { code: "ja", name: "日本語" },
        ],
        hideDefaultLocale: true,
      },
      ssg: {
        localeSwitcher: true,
        pagination: true,
        markdownSource: true,
        siteName: "cpg",
        siteUrl: `https://sevenc7c.com${base}`,
        theme: defineTheme({
          extends: terminal,
          sidebar: [
            { text: { en: "Introduction", ja: "はじめに" }, link: "/index.md" },
            {
              text: { en: "cpg vs oj + oj-prepare", ja: "oj・oj-prepare との比較" },
              link: "/cpg-vs-oj.md",
            },
            { text: { en: "Installation", ja: "インストール" }, link: "/installation.md" },
            {
              text: { en: "Configuration and login", ja: "設定とログイン" },
              link: "/configuration.md",
            },
            { text: { en: "Workspaces", ja: "ワークスペース" }, link: "/workspaces.md" },
            { text: { en: "Testing solutions", ja: "解答のテスト" }, link: "/testing.md" },
            {
              text: { en: "Generating test cases", ja: "テストケースの生成" },
              link: "/generating.md",
            },
            {
              text: { en: "Submissions and results", ja: "提出と結果の確認" },
              link: "/submissions.md",
            },
            {
              text: { en: "Command reference", ja: "コマンドリファレンス（英語）" },
              link: "/command-reference.md",
            },
          ],
          footer: {
            copyright: "2026 © Nanashi. Licensed under MIT License.",
          },
          socialLinks: {
            github: "https://github.com/sevenc-nanashi/competitive-programming-cli",
          },
        }),
      },
    }),
  ],
});
