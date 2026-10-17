---
layout: entry
title: Competitive Programming CLI (cpg)
description: 様々な競プロサイトに対応した、問題のダウンロード・テスト・提出状況の確認などの機能を備えたCLIツールです。
hero:
  text: モダン・高速・言語非依存の競技プログラミング用CLIツール
  tagline: 様々な競プロサイトに対応した、問題のダウンロード・テスト・提出状況の確認などの機能を備えたCLIツールです。
  notice:
    title: 注意
    body:
      - このプロジェクトは開発初期段階であり、まだ十分にテストされていません。
      - 設定やメタデータのフォーマットは変更される可能性があります。
  actions:
    - theme: brand
      text: インストール
      link: ./ja/installation.md
    - theme: alt
      text: ojとの比較
      link: ./ja/cpg-vs-oj.md
    - theme: alt
      text: GitHub
      link: https://github.com/sevenc-nanashi/competitive-programming-cli
features:
  - icon: "devicon:rust"
    title: Rust製
    details: |
      Rust製のシングルバイナリで配布されており、インストールが簡単で高速です。  
      (もちろん問題を解く際の言語はRustに限りません！）
    link: ./ja/installation.md
  - icon: "mdi:download"
    title: 問題・コンテストのダウンロード
    details: AtCoder、Codeforces、Yukicoder、ShareOJなど、様々な競プロサイトから問題をダウンロードできます。
    link: ./ja/workspaces.md
  - icon: "mdi:folder-multiple-outline"
    title: 強制されたディレクトリ構造
    details: ディレクトリ構造を固定することで、簡単に問題やコンテストを管理できます。
    link: "./ja/workspaces.md#workspace-root"
  - icon: "mdi:play-circle-outline"
    title: 解法のコンパイル＋テスト
    details: 解法をコンパイルしてテストできます。カスタムジャッジやインタラクティブ問題にも対応しています。
    link: ./ja/testing.md
  - icon: "mdi:dice-multiple-outline"
    title: テストケースの生成
    details: ランダムテスト・愚直解を用いたテストケースの生成が可能です。
    link: ./ja/generating.md
  - icon: "mdi:send"
    title: 提出＋結果の確認
    details: 解法を提出して、提出結果を確認できます。
    link: ./ja/submissions.md
  - icon: "mdi:clipboard-check-outline"
    title: クリップボード提出
    details: Captchaが必要なサイト向けに、クリップボード提出モードに切り替えられます。特にAtCoderの過去問で役にたちます。
    link: ./ja/submissions.md#copy-a-solution-to-the-clipboard
mdx: true
---

## デモ

<Asciinema src="/competitive-programming-cli/demo/overview.cast" poster="npt:11" />

## ガイド

1. [cpgをインストールする](./installation.md)。
2. [ワークスペースやCookieの設定をする](./configuration.md)。
3. [問題・コンテストをダウンロードする](./workspaces.md)。
4. [解法をテスト](./testing.md)したり[テストケースを生成](./generating.md)したりする。
5. [解法を提出して、結果を確認する](./submissions.md)。

[コマンドの一覧（英語）](../command-reference.md)も参照してください。
