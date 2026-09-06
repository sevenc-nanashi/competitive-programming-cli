import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

for (const [args, file] of [
  [["__usage_spec__"], "cpg.usage.kdl"],
  [["config", "--schema"], "public/config.schema.json"],
] as const) {
  const output = execFileSync(
    "cargo",
    ["run", "--quiet", "--locked", "--bin", "cpg", "--", ...args],
    {
      cwd: new URL("..", import.meta.url),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "inherit"],
    },
  );
  writeFileSync(new URL(file, import.meta.url), output);
}
execFileSync(
  "usage",
  ["generate", "markdown", "--file", "cpg.usage.kdl", "--out-file", "content/command-reference.md"],
  { cwd: import.meta.dirname, stdio: "inherit" },
);
