import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const spec = execFileSync(
  "cargo",
  ["run", "--quiet", "--locked", "--bin", "cpg", "--", "__usage_spec__"],
  {
    cwd: new URL("..", import.meta.url),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  },
);
writeFileSync(new URL("cpg.usage.kdl", import.meta.url), spec);
execFileSync(
  "usage",
  ["generate", "markdown", "--file", "cpg.usage.kdl", "--out-file", "content/command-reference.md"],
  { cwd: import.meta.dirname, stdio: "inherit" },
);
