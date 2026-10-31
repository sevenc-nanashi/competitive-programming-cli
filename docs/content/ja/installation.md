---
next: configuration.md
---

# インストール

cargo で cpg をインストールできます。

```bash
# NOTE: 現在はプレリリース版のみが提供されています。

# ソースからビルドする：
cargo install "competitive-programming-cli@>0.1.0-preview.0"

# cargo-binstall でビルド済みバイナリをインストールする：
cargo binstall "competitive-programming-cli@>0.1.0-preview.0"
```

リリースワークフローでビルドしたアーカイブと SHA-256 チェックサムは、[リリースページ](https://github.com/sevenc-nanashi/competitive-programming-cli/releases)で配布しています。
[mise](https://mise.en.dev) や [packslip](https://packslip.dev) を使ってインストールすることもできます。

```bash
# mise
mise use -g --tool-option prerelease=true "packslip:github.com/sevenc-nanashi/competitive-programming-cli" 

# packslip
packslip install github.com/sevenc-nanashi/competitive-programming-cli --pin ps1_iprtl2s5dcfnxce6p5udl6lnhi --version ">0.1.0-preview.0"
```

<a id="experimental-oj-backend"></a>

## oj バックエンド（実験的）

ネイティブバックエンドのないジャッジを使う場合は、`uv` をインストールして `PATH` を通してください。
初回利用時に cpg が `uv venv` を実行し、[online-judge-api-client のフォーク](https://github.com/sevenc-nanashi/online-judge-tools-api-client)を固定されたバージョンでインストールします。

使い方、仮想環境のパス、制限事項は[oj サポート](./configuration.md#experimental-oj-support)を参照してください。

<a id="shell-completion"></a>

## シェル補完

`cpg completion <shell>` は、cpg のコマンド定義から生成した補完スクリプトを出力します。
コマンド、エイリアス、フラグ、値の候補、ファイルやディレクトリのパスを補完できます。

Bash では `~/.bashrc` に次を追加してください。

```bash
source <(cpg completion bash)
```

Zsh では `~/.zshrc` で補完を有効にしてからスクリプトを読み込みます。

```zsh
autoload -Uz compinit
compinit
source <(cpg completion zsh)
```

Fish では補完用のディレクトリにスクリプトを保存してください。

```fish
mkdir -p ~/.config/fish/completions
cpg completion fish > ~/.config/fish/completions/cpg.fish
```

`elvish`、`nu`（Nushell）、`powershell` にも対応しています。生成されたスクリプトを保存し、シェルの設定から読み込んでください。
シェルを開き直すか設定を再読み込みすると、補完が有効になります。
