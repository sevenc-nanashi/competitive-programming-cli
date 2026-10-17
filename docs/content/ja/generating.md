# テストケースの生成

<a id="randomly-generate-test-cases"></a>

## ランダムなテストケースの生成

`generate` コマンドと生成用のスクリプトで、テストケースをランダムに生成できます。

```bash
# ジェネレーターを指定：
cpg generate ./random.rb

# ショートカット：
cpg g ./random.rb

# 明示的にディレクトリを指定：
cpg generate --dir ./random ./random.rb

# コマンドラインの全体を指定：
cpg generate -- ruby ./random.rb
```

デフォルトでは入力を100個生成します。`--count 10` を指定すると10個になります。
保存先のデフォルトは `./random` です。ファイル名は `random-0001.in` のような連番で、既存の `.in` や `.out` より後の番号を使います。

生成したテストケースに対して愚直解を実行し、想定出力を作成できます。

```bash
# 愚直解を指定：
cpg generate --answer ./naive.rb

# 短縮形：
cpg g -a ./naive.rb

# コマンドラインの全体を指定：
cpg generate --answer -- ruby ./naive.rb
```

`--answer` は、対応する `.out` がないすべての `.in` を処理します。既存の想定出力は変更しません。
`--count` とは併用できません。

`--jobs N`（`-j N`）を指定すると、生成スクリプトや愚直解を最大 N プロセスで並列実行できます。

```bash
cpg generate -j 4 --count 100 ./random.rb
cpg generate --answer -j 4 ./naive.rb
```
