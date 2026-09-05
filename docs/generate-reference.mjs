import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const spec = execFileSync(
  "cargo",
  ["run", "--quiet", "--locked", "--bin", "cpcli", "--", "__usage_spec__"],
  {
    cwd: new URL("..", import.meta.url),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  },
);
writeFileSync(new URL("cpcli.usage.kdl", import.meta.url), spec);
execFileSync(
  "usage",
  [
    "generate",
    "markdown",
    "--file",
    "cpcli.usage.kdl",
    "--out-file",
    "content/command-reference.md",
  ],
  { cwd: import.meta.dirname, stdio: "inherit" },
);
