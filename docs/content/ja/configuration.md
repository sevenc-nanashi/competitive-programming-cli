# 設定とログイン

<a id="configuration"></a>

## 設定

設定はデフォルトで `$XDG_CONFIG_HOME/cpg` に保存されます。このドキュメントではこのディレクトリを `$config` と呼びます。
環境変数 `$CPG_CONFIG_HOME` で保存先を変更できます。
XDG の環境変数が未設定の場合、設定ディレクトリは `~/.config/cpg`、データディレクトリは `~/.local/share/cpg` です。

まず初めにセットアップを実行してください。

```bash
cpg init
```

これで設定ファイルとディレクトリが作成されます。oj-prepare の設定ファイルがある場合は自動で取り込まれます。

`$config/config.toml` でワークスペースのルートを直接設定することもできます。

```toml
root = "/home/your-name/competitive-programming"
```

設定ファイルの JSON Schema は `https://raw.githubusercontent.com/sevenc-nanashi/competitive-programming-cli/refs/tags/v{version}/docs/public/config.schema.json` で公開しています。
ローカルでは `cpg config --schema` で出力できます。

設定ファイルに加え、次のディレクトリが作成されます。

- `$config/config.toml`: 設定ファイル。
- `$config/config.local.toml`: `config.toml` をマシンごとに上書きする任意の設定ファイル。
- `$config/workspace_template`: ワークスペースディレクトリのテンプレート。
  - この中のファイルやディレクトリを「ワークスペーステンプレート」と呼びます。
  - `Gemfile` や `Cargo.toml` など、ワークスペースのルートに置くファイルを配置します。
- `$config/problem_template`: 問題ディレクトリのテンプレート。
  - この中のファイルやディレクトリを「問題テンプレート」と呼びます。
  - `solve.cpp` や `naive.rb` など、1問分のファイルを配置します。
- `$config/contest_template`: コンテストディレクトリのテンプレート。
  - この中のファイルやディレクトリを「コンテストテンプレート」と呼びます。
  - ワークスペース設定を含む `Cargo.toml` など、コンテスト用のファイルを配置します。
- `$config/single_problem_template`: 単独の問題ディレクトリのテンプレート。
  - この中のファイルやディレクトリを「単独問題テンプレート」と呼びます。
  - 単独の問題ディレクトリだけで完結させるためのファイルを配置します。たとえば、 `Cargo.toml` をCargo workspaceを使わない問題用に差し替えることができます。

各ディレクトリのパスは `cpg config` で確認できます。

```bash
cpg config
# Workspace root: /home/your-name/competitive-programming
# Configuration directory: /home/your-name/.config/cpg
# Cookies directory: /home/your-name/.local/share/cpg/cookies
# oj virtual environment directory: /home/your-name/.local/share/cpg/oj-venv
# Workspace template directory: /home/your-name/.config/cpg/workspace_template
# Problem template directory: /home/your-name/.config/cpg/problem_template
# Contest template directory: /home/your-name/.config/cpg/contest_template
# Single problem template directory: /home/your-name/.config/cpg/single_problem_template

# ワークスペースのルートを表示：
cpg config --root

# 他のディレクトリのパスを表示：
cpg config --config-dir
cpg config --cookies-dir
cpg config --oj-venv-dir
cpg config --workspace-template-dir
cpg config --problem-template-dir
cpg config --contest-template-dir
cpg config --single-problem-template-dir
```

<a id="local-overrides"></a>

## 環境ごとの設定の上書き

`config.toml` と同じ場所に `config.local.toml` を作成すると、`config.toml` の設定を上書きできます。
`$CPG_CONFIG_HOME` が設定されていれば両方のファイルをそこから読み込み、未設定なら前述の設定ディレクトリから読み込みます。

```toml
# config.local.toml
root = "~/local-competitive-programming"

[language.cpp]
compile = "clang++ -std=c++23 -O2 -o {binary} {input}"

[clipboard]
kind = "command"
command = "/mnt/c/Windows/System32/clip.exe"
```

特に複数のマシンで cpg を使う場合、`config.local.toml` にマシン固有の設定を置くと便利です。

<a id="logging"></a>

## ログ

`CPG_LOG` には [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) の `EnvFilter` 構文を指定できます。
ログは標準出力には出力しません。

```bash
CPG_LOG=debug cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f
CPG_LOG=info,cpg=debug,reqwest=warn cpg prepare https://atcoder.jp/contests/abc473/tasks/abc473_f
CPG_LOG=off cpg list
```

<a id="directory-prefixes"></a>

## ディレクトリ名の接頭辞

コンテスト内の問題ディレクトリの接頭辞に英小文字を使うには、`config.toml` または `config.local.toml` のトップレベルに次を追加してください。

```toml
alphabetic = true
```

接頭辞は `a`、…、`z`、`aa`、…の順になります。
27問なら、ディレクトリ名は `_a_`、…、`_z_`、`aa_` のように、 `_` を余白として  
デフォルトは `false` で、ゼロ埋めした数字を使います。

<a id="clipboard"></a>

## クリップボード

`cpg submit --clipboard` は、`preprocess` と `presubmit` の実行後のソースをコピーします。
`$config/config.toml` の `[clipboard]` でバックエンドを選べます。

```toml
[clipboard]
kind = "osc52"
```

セクションを省略した場合のデフォルトは `osc52` です。
標準エラー出力に OSC 52 エスケープシーケンスを送り、ターミナルにテキストのコピーを要求します。
ターミナルが OSC 52 に対応し、クリップボードへの書き込みを許可している必要があります。また、標準エラー出力の接続先がターミナルでなければなりません。

デフォルトでは`cpg submit --clipboard` が失敗するとクリップボードを空にします。
これはうっかり別の問題や無効なコードを提出してしまうのを防ぐためです。（クリップボードが空になる -> 空のものを提出しようとする -> ほとんどのジャッジは空の提出を拒否する）
この動作は次の設定で無効にできます。

```toml
[clipboard]
clear_on_failure = false
```

[arboard](https://docs.rs/arboard/latest/arboard/) 経由でシステムのクリップボードを使う場合は、次のように設定します。

```toml
[clipboard]
kind = "arboard"
```

Linux では、cpg の終了後もクリップボードマネージャーがテキストを保持する必要があります。
保持されない場合は、後述の `wl-copy` や `xclip` などを使ってください。

特定のコマンドを使う場合は、次のように設定します。

```toml
[clipboard]
kind = "command"
command = "wl-copy"
```

このバックエンドでは `command` が必須です。カレントディレクトリで `sh -c`（Windows では `cmd /C`）を通して実行します。
UTF-8 のテキスト全体を改行を追加せずコマンドの標準入力に渡します。
コマンドの標準出力と標準エラー出力は cpg の標準エラー出力に送られます。
終了コードが `0` 以外ならコピー失敗として扱われます。

例えば X11 では `command = "xclip -selection clipboard"` 、 WSL では `command = "/mnt/c/Windows/System32/clip.exe"` のように設定できます。

<a id="remote-mode"></a>

## リモートモード

SSH 越しに cpg を使うとき用に `--remote` オプションがあります。リモートモードでは次の動作をします。

- クリップボードのバックエンドを `osc52` にします。
- `--open` を指定した場合、ブラウザーを開く代わりに URL をコピーします。

デフォルトでは次のいずれかに当てはまると自動で有効になります。

- SSH の環境変数を検出した場合。
- `CPG_REMOTE` が `1` の場合。
- コマンドラインで `--remote` を指定した場合。

SSH の検出は`$config/config.toml` の次の設定で無効にできます。

```toml
remote = "manual"
```

<a id="templates-and-setup-commands"></a>

## テンプレートとセットアップコマンド

`$config/workspace_template`、`$config/problem_template`、`$config/contest_template`、`$config/single_problem_template` にテンプレートを置くと、問題やコンテストのディレクトリにコピーされます。

`$config/config.toml` に `[setup]` を追加すると、テンプレートのコピー後にコマンドを実行できます。

```toml
[setup]
workspace = ["git init", "bundle install"]
problem = "ruby setup_problem.rb"
contest = "ruby setup_contest.rb"
single_problem = "ruby setup_single_problem.rb"
```

各キーにはコマンドまたはコマンドの配列を指定できます。
スクリプトは対応するテンプレートディレクトリに置いてください。ファイルコピーはテンプレートのコピー後に行われます。
テンプレートディレクトリが存在しなくてもコマンドは実行されるため、コマンドだけでセットアップすることもできます。

| キー             | テンプレート              | 作業ディレクトリ               |
| ---------------- | ------------------------- | ------------------------------ |
| `workspace`      | `workspace_template`      | 単独の問題やコンテストのルート |
| `problem`        | `problem_template`        | 各問題のディレクトリ           |
| `contest`        | `contest_template`        | コンテストのルート             |
| `single_problem` | `single_problem_template` | 単独の問題のルート             |

テンプレートのコピーとセットアップは次の順で行います。

- 問題やコンテストのメタデータを `.cpg.toml` に書き込む。
- `workspace_template` をワークスペースのルートにコピーする。
- コンテストの場合、`contest_template` をコンテストのルートにコピーする。
- `problem_template` を各問題のディレクトリにコピーする。
- 単独の問題の場合、`single_problem_template` を問題のルートにコピーする。
- 各問題のディレクトリで `[setup.problem]` を実行する。
- ワークスペースのルートで `[setup.workspace]` を実行する。
- コンテストの場合、コンテストのルートで `[setup.contest]` を実行する。
- 単独の問題の場合、問題のルートで `[setup.single_problem]` を実行する。

同じファイルが複数のテンプレートに存在する場合、下にあるテンプレートのファイルで上書きされます。

例えば `bundle install` や `git clone https://github.com/atcoder/ac-library.git` を `[setup.workspace]` に置くと良いでしょう。

<a id="login"></a>

## ログイン

cpg 自体はログインフォームを実装していません。
オンラインジャッジごとに Cookie を用意し、Netscape HTTP Cookie Format で保存する必要があります。
[Get cookies.txt LOCALLY](https://github.com/kairi003/Get-cookies.txt-LOCALLY) 拡張機能を使うとよいでしょう。

Cookie ファイルの保存先はデフォルトで `$XDG_DATA_HOME/cpg/cookies`、パーミッションは `600` です。

```bash
# Login to an online judge
cpg login atcoder --cookie-file /path/to/cookies.txt
```

`--info` で保存したセッションを検証し、ユーザー名とプロフィール URL を表示できます。

```bash
cpg login atcoder --info
cpg login yukicoder --info
cpg login codeforces --info
cpg login share-oj --info
cpg login atcoder-problems --info
```

標準出力の1行目にユーザー名、2行目にプロフィール URL を出力します。

<a id="experimental-oj-support"></a>

## oj サポート（実験的）

ネイティブバックエンドが対応していない URL は、自動で [online-judge-api-client のフォーク](https://github.com/sevenc-nanashi/online-judge-tools-api-client)に渡されます。
対応する URL や操作はクライアントとジャッジによって異なります。
例えば以下のようにすると Aizu Online Judge の問題をダウンロードできます。

```bash
cpg download https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A
```

`uv` をインストールして `PATH` に追加しておく必要があります。
初回利用時に cpg が `$XDG_DATA_HOME/cpg/oj-venv` に Python の仮想環境を作成します。
デフォルトのパスは `~/.local/share/cpg/oj-venv` です。

oj バックエンドのサービス名は `oj+<host>` です。たとえば Aizu Online Judge は `oj+onlinejudge.u-aizu.ac.jp` になります。

```bash
cpg login oj+onlinejudge.u-aizu.ac.jp --cookie-file /path/to/cookies.txt
```

このバックエンドは実験的なものです。ジャッジによってはログインの検証や提出に失敗したり、対応していなかったりする場合があります。
`cpg results` には対応していません。コンテストのダウンロードはネイティブバックエンドより遅いかもしれません。

<a id="language-settings"></a>

## 言語設定

`$config/config.toml` に `[language.<name>]` テーブルを追加してください。
cpg はソースファイルの拡張子で言語を選びます。この設定は解答、生成スクリプト、愚直解、ジャッジのファイルに適用されます。

| キー         | 必須   | 用途                                                         |
| ------------ | ------ | ------------------------------------------------------------ |
| `extensions` | はい   | `["cpp", "cc"]` のような、先頭のドットを除いた拡張子。       |
| `run`        | はい   | 解答やスクリプトを実行するシェルコマンド。                   |
| `compile`    | いいえ | 実行前にソースをコンパイルまたは検査するシェルコマンド。     |
| `preprocess` | いいえ | ローカルでの実行前と提出前にソースを変換するコマンド。       |
| `presubmit`  | いいえ | 提出時だけ、`preprocess` の後にソースを変換するコマンド。    |
| `profile`    | いいえ | `--profile` で選択する、コマンドや提出言語 ID の上書き設定。 |
| `submit`     | いいえ | `atcoder`、`yukicoder` など、サービスごとの提出言語 ID。     |

`compile` と `run` では、`{input}` はソースのパスに展開されます。
`{binary}` は、Linux/macOS ではソースの最後の拡張子を取り除いたパス、Windows では `.exe` に置き換えたパスになります。
プレースホルダーは自動でシェル用にクォートされるため、引用符で囲まないでください。

前処理を設定した場合、`{input}` は変換後のソースを指すようになります。
`--` の後に直接指定したコマンドには、この言語設定を適用しません。

プロファイルで上書きしたものも含め、すべての言語コマンド（`compile`、`run`、`preprocess`、`presubmit`）では、次のプレースホルダーを使えます。

- `{workspace}`: コンテストのルートまたは単独の問題ディレクトリの絶対パス。
- `{problem}`: 問題ディレクトリの絶対パス。

```toml
[language.cpp]
extensions = ["cpp"]
compile = "g++ -std=c++23 -I{workspace}/ac-library -I{problem}/include -o {binary} {input}"
run = "{binary}"
```

<a id="build-profiles"></a>

### ビルドプロファイル

`[language.<name>.profile.<profile>]` でコマンドを上書きできます。
（コマンドやフラグを追加するのではなく、完全に置き換えます。）
空文字列（`""`）または `false` で、継承したコマンドを無効にできます。

```toml
[language.python.profile.plain]
preprocess = false
presubmit = ""
compile = false
```

たとえば、インタープリターと提出言語をまとめて変更できます。

```toml
[language.python.profile.pypy]
run = "pypy3 {input}"

[language.python.profile.pypy.submit]
yukicoder = "pypy3"
```

明示的に指定した `--language ID` はプロファイルの `submit` 設定より優先されます。

<a id="executable-files"></a>

### 直接実行

設定済みの拡張子に一致しない実行可能ファイルは組み込みの `executable` 言語として扱われます。
この言語はコンパイルせず `{input}` を実行します。
Linux/macOS では拡張子のないバイナリや、実行権限と shebang のあるスクリプト、Windows では `.exe` や `.com` ファイルが対象です。

```bash
cpg test ./a.out
cpg generate --count 10 ./generator
```

この場合の動作を変更するには、`config.toml` に `[language.executable]` を定義してください。

```toml
[language.executable]
extensions = []
run = "{input}"

[language.executable.profile.debug]
run = "env DEBUG=1 {input}"
```

<a id="source-transformations"></a>

### ソースの変換

任意の `language.<name>.preprocess` はコンパイル前（インタープリター言語では実行前）と提出前に実行されます。
`language.<name>.presubmit` は、提出時だけ `preprocess` の後に実行されます。

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "ruby ~/expand.rb {input} {processed}"
compile = "g++ -std=c++23 -O2 -o {binary} {input}"
run = "{binary}"
```

`presubmit` でも同じプレースホルダーを使えます。
提出時だけ変換したい場合は、`preprocess` の代わりに `presubmit` を使ってください。
[ACL の設定例](#expand-atcoder-library-headers)も参照してください。

<a id="submission-language-ids"></a>

### 提出言語 ID

既存の言語の `submit` テーブルに、ジャッジの言語 ID を追加してください。
例えば C++23 は以下のようにします：

```toml
[language.cpp.submit]
atcoder = "6017"
yukicoder = "cpp23"
```

ID は数字の場合も引用符で囲んだ文字列にする必要があります。
言語 ID を設定せずに `cpg submit` で提出しようとすると、利用可能な ID を確認できます。
プロファイルの `submit` テーブルでも上書きできます。
[解答の提出](./submissions.md)も参照してください。

<a id="recipes"></a>

## 設定例

必要な設定を `$config/config.toml` にコピーし、コマンドで使うコンパイラーやインタープリターをインストールしてください。

<a id="c-with-debugging-and-a-fast-profile"></a>

### C++ のデバッグ設定と fast プロファイル

この設定は GCC と C++23 を使います。
デフォルトのビルドではデバッグシンボルと [UndefinedBehaviorSanitizer](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html) を有効にします。
`-fno-sanitize-recover=all` により、エラーを検出した時点でプログラムが停止し、テスト結果が `RE` になります。
`fast` プロファイルは最適化を有効にし、`ONLINE_JUDGE` を定義します。

```toml
[language.cpp]
extensions = ["cpp", "cc"]
compile = "g++ -std=c++23 -O0 -g -Wall -Wextra -fsanitize=undefined -fno-sanitize-recover=all -o {binary} {input}"
run = "{binary}"

[language.cpp.profile.fast]
compile = "g++ -std=c++23 -O2 -Wall -Wextra -DONLINE_JUDGE -o {binary} {input}"

[language.cpp.submit]
atcoder = "6017" # C++23 (GCC 15.2.0)
yukicoder = "cpp23"
# clang を使う場合は次のようにします：
#
# atcoder = "6116" # C++23 (Clang 21.1.0)
# yukicoder = "cpp-clang"
#
# ただし yukicoder 上の Clang は C++17 なので、コンパイルコマンドの `-std=c++23` を `-std=c++17` に変更する必要があリます。
```

### Ruby

コンパイルコマンドは `ruby -c` で構文を検査します。実行コマンドはスクリプトを実行します。

```toml
[language.ruby]
extensions = ["rb"]
compile = "ruby -c {input}"
run = "ruby {input}"

[language.ruby.submit]
atcoder = "6087" # Ruby 3.4 (ruby 3.4.5)
yukicoder = "ruby"
```

### Python

実行コマンドは Python 3 でスクリプトを実行します。

```toml
[language.python]
extensions = ["py"]
run = "python3 {input}"

[language.python.submit]
atcoder = "6083" # PyPy 3.11-v7.3.20
yukicoder = "pypy3"

# どうしても CPython を使いたい場合：
#
# atcoder = "6082" # Python (CPython 3.13.7)
# yukicoder = "python3"
```

<a id="expand-atcoder-library-headers"></a>

### AtCoder Library のヘッダーの展開

[ac-library](https://github.com/atcoder/ac-library) を `~/ac-library` にチェックアウトしておき、[expander](https://github.com/atcoder/ac-library/blob/master/expander.py) でローカルでのコンパイル前と提出前に ACL のヘッダーをソース内に展開します。
Python 3 と GCC が必要です。

```toml
[language.cpp]
extensions = ["cpp"]
preprocess = "python3 ~/ac-library/expander.py --console --lib ~/ac-library {input} > {processed}"
compile = "g++ -std=c++23 -O2 -o {binary} {input}"
run = "{binary}"

[language.cpp.submit]
atcoder = "6017" # C++23 (GCC 15.2.0)
yukicoder = "cpp23"
```

提出時だけ展開するには、`preprocess` を `presubmit` に置き換えることができます。
ただし、ローカルのテストでは展開せずにヘッダーを読み込めるよう、コンパイルコマンドに `-I "$HOME/ac-library"` を追加する必要があります。
