---
layout: entry
title: Competitive Programming CLI (cpg)
description: A command-line interface tool for competitive programming, supporting multiple online judges and features like problem downloading, solution testing, and submission tracking.
hero:
  text: Modern, fast, and language-agnostic tool for Competitive Programming
  tagline: A command-line interface tool for competitive programming, supporting multiple online judges and features like problem downloading, solution testing, and submission tracking.
  notice:
    title: Early development
    body:
      - This project is in early development, and is not well-tested yet.
      - Configuration and metadata formats may change.
  actions:
    - theme: brand
      text: Get Started
      link: ./installation.md
    - theme: alt
      text: Compare with oj
      link: ./cpg-vs-oj.md
    - theme: alt
      text: View on GitHub
      link: https://github.com/sevenc-nanashi/competitive-programming-cli
features:
  - icon: "devicon:rust"
    title: Written in Rust
    details: |
      cpg is written in Rust, making it fast and easy to install.  
      (Don't worry, you can still use your favorite programming language for solving problems!)
    link: installation.md
  - icon: "mdi:download"
    title: Download Problems and Contests
    details: Download problems from various online judges, including AtCoder, Codeforces, Yukicoder, ShareOJ, and more.
    link: workspaces.md
  - icon: "mdi:folder-multiple-outline"
    title: Fixed Directory Structure
    details: Force directory structure for problems and contests for effortless management of problems and contests.
    link: "workspaces.md#workspace-root"
  - icon: "mdi:play-circle-outline"
    title: Compile and Test Solutions
    details: Compile and test your solutions, including support for custom and interactive judges.
    link: testing.md
  - icon: "mdi:dice-multiple-outline"
    title: Generate Test Cases and Reference Answers
    details: Generate test cases and reference answers using your own generator and naive solution.
    link: generating.md
  - icon: "mdi:send"
    title: Submit Solutions and Watch Results
    details: Submit your solutions to online judges and watch the results in real-time.
    link: submissions.md
  - icon: "mdi:clipboard-check-outline"
    title: Copy submission to Clipboard
    details: Switch to clipboard submission mode for judges that require captcha verification, such as AtCoder's past contests.
    link: submissions.md#copy-a-solution-to-the-clipboard
mdx: true
---

## Demo

<Asciinema src="/competitive-programming-cli/demo/overview.cast" poster="npt:11" />

## Getting started

1. [Install cpg](./installation.md).
2. [Configure your workspace and log in](./configuration.md).
3. [Download a problem or contest](./workspaces.md).
4. [Test your solution](./testing.md) and [generate extra test cases](./generating.md).
5. [Submit your solution and watch the results](./submissions.md).

See the [command reference](./command-reference.md) for all commands and flags.
