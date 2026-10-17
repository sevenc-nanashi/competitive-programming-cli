# cpg vs oj + oj-prepare

[online-judge-tools/oj](https://github.com/online-judge-tools/oj) と、[online-judge-tools/template-generator](https://github.com/online-judge-tools/template-generator) （oj-prepare）は
競技プログラミング用ツールとして長らく使われてきました。

開発者自身も長く使ってきましたが、ワークスペースの管理、解答のコンパイル、提出の確認をまとめて行いたくなったため、cpg を作りました。

<a id="what-changes-with-cpg"></a>

## oj との違い

- Rust 製のシングルバイナリのため、インストールや更新がとても簡単です。
  - （ただし [oj バックエンド](./configuration.md#experimental-oj-support)は `uv` が必要です）
- 問題やコンテストを固定のディレクトリ下に配置します。
  - ダウンロード先に悩まずに済むので、問題を解き始めるハードルも下がるかもしれません。
    - 開発者は下がりました。
- コンテスト内の問題ディレクトリ名に番号を追加しています。
  - 問題が ID 順に並ばない場合、例えばバーチャルコンテストや Yukicoder のコンテストで便利です。
- 単独の問題とコンテストで共通のテンプレートを使えます。
- 1つのコマンドでコンパイルとテストができます。
  - 言語ごとにコマンドやビルドプロファイルが必要です。
  - 解答の実行に加え、`preprocess` や `presubmit` でテスト前や提出前にソースを変換できます。
- `cpg results --ui` で自分の提出を監視できます。
  - 提出ページを開かずに結果を確認できます。

<a id="command-equivalents"></a>

## コマンドの対応表

次の例は [oj のコマンド](https://github.com/online-judge-tools/oj#how-to-use) と [oj-prepare](https://github.com/online-judge-tools/template-generator#usage) を使っています。
ソースを渡すタイプの cpg のコマンドを使う場合は、先にその言語のコンパイル・実行コマンドを設定してください。
`--` の後にコマンドを直接指定する場合、言語設定は不要です。

| 操作                         | oj / oj-prepare                                          | cpg                                                     |
| ---------------------------- | -------------------------------------------------------- | ------------------------------------------------------- |
| 問題のサンプルをダウンロード | `oj download URL`                                        | `cpg download URL`                                      |
| コンテストや問題の準備       | `oj-prepare URL`                                         | `cpg prepare URL`                                       |
| C++ のコンパイルとテスト     | `g++ solution.cpp -o solution && oj test -c ./solution`  | `cpg test solution.cpp`<sup>1</sup>                     |
| Ruby のテスト                | `oj test -c "ruby solution.rb"`                          | `cpg test solution.rb`                                  |
| 指定した入力ファイルでテスト | `oj test -c "ruby solution.rb" test/sample-1.in`         | `cpg test --input-path ./test/sample-1.in solution.rb`  |
| `test/` に入力を生成         | `oj generate-input "ruby random.rb"`                     | `cpg generate random.rb`<sup>2</sup>                    |
| `test/` 内に回答例を生成     | `oj generate-output -c "ruby naive.rb"`                  | `cpg generate --answer naive.rb`                        |
| インタラクティブ問題のテスト | `oj test-reactive -c "ruby solution.rb" "ruby judge.rb"` | `cpg test --interactive --judge "judge.rb" solution.rb` |
| 指定した問題に提出           | `oj submit URL solution.cpp`                             | `cpg submit solution.cpp --problem URL`                 |

1. cpg は1つのコマンドでコンパイルとテストができますが、コマンドを直接指定する方法（`cpg test -- ruby solution.rb`）や、先にコンパイルする方法（`g++ solution.cpp -o solution && cpg test ./solution`）も使えます。
2. cpg はデフォルトで `random/` にランダムケースを生成します。`--dir` で `test/` など別の保存先を指定できます。

<a id="differences-when-switching"></a>

## 移行時の違い

- コマンドラインオプションが異なります。[コマンドリファレンス（英語）](../command-reference.md)を参照してください。
- テンプレートファイルを処理するテンプレートシステムはありません。ファイルをそのままコピーします。
- `oj-template` に相当する機能はありません。
- `oj download --system` に相当する機能はありません。

<a id="migrating-templates"></a>

## テンプレートの移行

`cpg init` は、`online-judge-tools` の XDG 設定ディレクトリにある `prepare.config.toml` を検出し、`[templates]` の項目を取り込むか確認します。
取り込みでは参照先のローカルファイルを `problem_template` にコピーします。

取り込み後は[言語設定](./configuration.md#language-settings)を追加し、[ブラウザーの Cookie を取り込む](./configuration.md#login)必要があります。
コンテストのルートにファイルを置きたい場合は、`workspace_template` に移動してください。

既存の oj ワークスペースは変換しませんが、`.in` と `.out` ファイルはテストに使うことができます。

```bash
cpg test --test-dir ./test -- ruby ./solution.rb
```

`.cpg.toml` のないディレクトリから提出する場合は、`--problem URL` が必要です。
