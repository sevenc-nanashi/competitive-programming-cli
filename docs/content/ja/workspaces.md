# ワークスペース

<a id="download-sample-cases"></a>

## サンプルケースのダウンロード

`cpg download`（`cpg d`）は、1問分のサンプルを `./test/` にダウンロードします。
ワークスペースのルートは不要です。テンプレートのコピー、セットアップコマンドの実行、メタデータの書き込みも行いません。
別の保存先を使う場合は `-d` / `--directory` で指定してください。既存のサンプルファイルは上書きせず、ディレクトリ内のほかのファイルもそのまま残します。

```bash
cpg d https://atcoder.jp/contests/abc473/tasks/abc473_f
cpg d https://atcoder.jp/contests/abc473/tasks/abc473_f -d samples
```

<a id="workspace-root"></a>

## ワークスペースのルート

単独の問題やコンテストの問題をダウンロードしたディレクトリを「ワークスペース」と呼びます。

ワークスペースは`~/.config/cpg/config.toml` で設定したワークスペースのルートに配置されます。  
たとえばルートが `~/competitive-programming` の場合、AtCoder の単独の問題は `~/competitive-programming/atcoder/problems/abc473_f`、コンテストは `~/competitive-programming/atcoder/contests/abc473` に配置されます。  
この機能は [ghq](https://github.com/x-motemen/ghq) に強く影響を受けています（ghq は `$root/$host/$path` の形式で Git リポジトリを管理するツールで、たとえば `~/ghq/github.com/sevenc-nanashi/competitive-programming-cli` のように配置されます）。

cpg をしばらく使うと、ワークスペースのルートは以下のようになるでしょう：

```file-tree
- ~/cpg/
  - atcoder/
    - contests/
      - aalc001/
      - abc465/
      - abc466/
      - abc467/
      - .../
      - adt_all_20260908_1/
      - adt_all_20260909_1/
      - .../
    - problems/
      - abc294_g/
      - abc313_d/
      - .../
  - yukicoder/
    - contests/
      - 594/
      - .../
  - share-oj/
    - contests/
      - dc6fc9dd-4019-463a-a486-12c3fc10af59/
    - problems/
      - 898f6783-900e-4035-9a03-500571a20d06/
      - 796156e7-bc25-4b68-82ec-494135b75dc5/
      - .../
  - .../
```

<a id="prepare-a-single-problem"></a>

## 単独の問題のセットアップ

コンテストから1問だけダウンロードできます。
問題のディレクトリには次のファイルが配置されます。

- 問題のメタデータを含む `.cpg.toml`。
- ワークスペーステンプレートのファイル。
- 問題テンプレートのファイル。
- 単独問題テンプレートのファイル。
- テストケース（`test/sample-1.in` と `test/sample-1.out` に保存）。

サンプルの番号は1つしかない場合も必ず1から始まります。
2つ目以降は `sample-2.in`、`sample-2.out` のような名前になります。
`.cpg.toml` には、サービス、問題 ID、正規 URL、タイトルに加え、該当する場合は元のコンテスト ID や内部 ID を記録します。

テンプレートに関しては [テンプレートとセットアップコマンド](./configuration.md#templates-and-setup-commands) を参照してください。

```bash
# AtCoder から問題をダウンロードしてセットアップする：
cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f

# 短縮形：
cpg p https://atcoder.jp/contests/abc473/tasks/abc473_f
```

単独の問題のディレクトリは以下のようになります：

```file-tree
- abc473_f/
  - .cpg.toml
  - ac-library/ **
  - test/
    - sample-1.in
    - sample-1.out
    - sample-2.in
    - sample-2.out
    - sample-3.in
    - sample-3.out
    - sample-4.in
    - sample-4.out
  - Gemfile **
  - Gemfile.lock **
  - compile_flags.txt **
  - judge.rb **
  - main.cpp **
  - main.rb **
  - naive.rb **
```

<a id="prepare-contest-problems"></a>

## コンテストのセットアップ

コンテストの全問題もダウンロードできます。
コンテストのディレクトリには次のファイルが配置されます。

- コンテストのメタデータを含む `.cpg.toml`。
- ワークスペーステンプレートのファイル。
- コンテストテンプレートのファイル。

各問題のディレクトリには次のファイルが配置されます。

- 問題テンプレートのファイル。
- 問題のメタデータとサンプルケース。
- （ワークスペーステンプレートのファイルは含まれません）

コンテストのメタデータには問題の順序を保った一覧を記録します。

テンプレートに関しては [テンプレートとセットアップコマンド](./configuration.md#templates-and-setup-commands) を参照してください。

```bash
# AtCoder からコンテストをダウンロードしてセットアップする：
cpg prepare https://atcoder.jp/contests/abc473

# 短縮形：
cpg p https://atcoder.jp/contests/abc473
```

コンテストのディレクトリは以下のようになります：

```file-tree
- abc479/
  - .cpg.toml
  - ac-library/ **
  - 1_abc479_a/
    - .cpg.toml
    - test/
      - sample-1.in
      - sample-1.out
      - sample-2.in
      - sample-2.out
      - sample-3.in
      - sample-3.out
    - main.cpp **
    - main.rb **
    - naive.rb **
    - judge.rb **
  - 2_abc479_b/
  - 3_abc479_c/
  - 4_abc479_d/
  - 5_abc479_e/
  - 6_abc479_f/
  - 7_abc479_g/
  - Gemfile **
  - Gemfile.lock **
  - compile_flags.txt **
```

<a id="open-the-current-problem-or-contest"></a>

## 現在の問題やコンテストをブラウザで開く

```bash
cpg open
# Or shorthand
cpg o
```

最も近い `.cpg.toml` に記録された問題やコンテストの URL を、デフォルトのブラウザーで開きます。
サブディレクトリからも実行できます。コンテスト内の問題ディレクトリからはその問題を、コンテストのディレクトリからはコンテストのページを開きます。

`--url-only` を指定すると、ブラウザーを開かず URL だけを標準出力に出力します。

```bash
cpg open --url-only
# 短縮形：
cpg o -n
```

`--clipboard` を指定すると、ブラウザーを開く代わりに URL をクリップボードにコピーします。

```bash
cpg open --clipboard
# 短縮形：
cpg o -c
```

<a id="list-workspaces"></a>

## ワークスペースの一覧

`list` コマンドで、ダウンロード済みのワークスペースを一覧表示できます。

デフォルトではコンテストと単独の問題のワークスペースディレクトリを表示します。
パスはワークスペースのルートからの相対パスです。ルートは `cpg config --root` で確認できます。
次のフィルターのうち、いずれか1つを指定できます。

| オプション                  | 表示するディレクトリ             |
| --------------------------- | -------------------------------- |
| `--workspace`（デフォルト） | コンテストと単独の問題           |
| `--contests`                | コンテスト                       |
| `--problems`                | 単独の問題                       |
| `--all-problems`            | 単独の問題とコンテスト内の各問題 |

```bash
# ワークスペース一覧を出す：
cpg list

# ワークスペースのルートを出す：
cpg config --root

# コンテストを出す：
cpg list --contests

# 単独の問題を出す：
cpg list --problems

# コンテスト内の各問題も含めて出す：
cpg list --all-problems
```

`--format`（`-f`）で各行の出力形式を変更できます。

| プレースホルダー | 内容                                 |
| ---------------- | ------------------------------------ |
| `{path}`         | ワークスペースのルートからの相対パス |
| `{service}`      | オンラインジャッジのサービス名       |
| `{id}`           | 問題やコンテストの ID                |
| `{title}`        | 問題やコンテストのタイトル           |
| `{url}`          | 問題やコンテストの URL               |
| `\t`             | タブ文字                             |

```bash
cpg list --format '{path}\t{path} ({title})'
```

`--delimiter` で各レコードの末尾に付ける区切り文字を指定できます。
`line`（デフォルトの改行）、`null`（NUL）、`tab`、または任意の1文字を指定してください。
たとえば NUL 区切りにすると `fzf --read0` に渡せるようになります。

```bash
cpg list -f '{path}\t{path} ({title})' --delimiter null |
    fzf --read0 --delimiter=$'\t' --with-nth=2..
```

このコマンドは`fzf` などのコマンドに出力をパイプで渡す用途を想定しています。

より複雑な整形が必要な場合は、`--json` で各レコードを JSON として出力し、`jq` などの JSON ツールで処理できます。

選択した問題のディレクトリへ移動する `ccd` コマンドを作ることもできます：

```bash
ccd() {
    local root dir
    root="$(CPG_LOG=none cpg config --root)" || return
    dir="$(CPG_LOG=none cpg list -f '{path}\t{path} ({title})' | fzf --delimiter=$'\t' --with-nth=2..)" || return
    dir="${dir%%$'\t'*}"
    [ -n "$dir" ] && cd -- "$root/$dir"
}
```
