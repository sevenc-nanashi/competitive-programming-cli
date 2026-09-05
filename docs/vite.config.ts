import { defineConfig } from "vite";
import { defineTheme, oxContent } from "@ox-content/vite-plugin";

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
      ssg: {
        siteName: "cpcli",
        siteUrl: `https://sevenc-nanashi.github.io${base}`,
        theme: defineTheme({
          sidebar: [
            { text: "Introduction", link: "/index.md" },
            { text: "cpcli vs oj", link: "/cpcli-vs-oj.md" },
            { text: "Installation", link: "/installation.md" },
            { text: "Configuration and login", link: "/configuration.md" },
            { text: "Workspaces", link: "/workspaces.md" },
            { text: "Testing solutions", link: "/testing.md" },
            { text: "Generating test cases", link: "/generating.md" },
            { text: "Submissions and results", link: "/submissions.md" },
          ],
          socialLinks: {
            github: "https://github.com/sevenc-nanashi/competitive-programming-cli",
          },
        }),
      },
    }),
  ],
});
