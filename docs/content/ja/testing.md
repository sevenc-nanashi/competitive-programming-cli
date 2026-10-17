# 解答のテスト

<Asciinema src="/competitive-programming-cli/demo/testing.cast" poster="npt:15" />

<a id="test-a-solution"></a>

## 解答をテストする

`test` コマンドで、サンプルケースを使って解答をテストできます。

```bash
# サンプルケースを使って解答をテスト：
cpg test ./solution.cpp

# 短縮形：
cpg t ./solution.cpp

# テストケースのディレクトリを指定する：
cpg test --test-dir ./random ./solution.cpp

# プロファイルを指定する：
cpg test --profile fast ./solution.cpp
```

`--` の後ろに実行するコマンドを直接指定することもできます。

```bash
# コマンドを直接指定して解答をテスト：
cpg test -- ruby ./solution.rb
```

TLE や MLE を判定するには、`--time-limit` と `--memory-limit` で各ケースの実行時間とメモリの制限を指定できます。

```bash
# TL を 2 秒、ML を 256 MB に設定して解答をテスト：
cpg test --time-limit 2000 --memory-limit 256 ./solution.cpp
```

これらの制限は厳密ではなく、システムの負荷などによって変動することがあります。

`--jobs N`（`-j N`）で最大 N ケースを並列実行できます。デフォルトは `1` です。

```bash
cpg test -j 4 ./solution.cpp
```

`--fast-fail`（`-f`）を指定すると、最初の失敗以降は新しいケースの実行を開始しなくなります。

```bash
# 最初に失敗したケースでテストを中止：
cpg test --fast-fail ./solution.cpp
```

浮動小数点数の比較では`--float-error` で許容誤差を指定できます。
想定出力と実際の出力の絶対誤差または相対誤差が、指定した値以下なら許容します。

```bash
# 小数点誤差を 10^-6 まで許容して解答をテスト：
cpg test --float-error 1e-6 ./solution.cpp

# 絶対誤差のみを許容する場合：
cpg test --float-error 1e-6 --float-error-type absolute ./solution.cpp

# 相対誤差のみを許容する場合：
cpg test --float-error 1e-6 --float-error-type relative ./solution.cpp
```

カスタムジャッジを使う場合は、`--judge`（`-J`）で各ケースの判定に使うコマンドを指定してください。
ジャッジには3つの引数を渡します。`{test_input}` は `.in` のパス、`{solution_output}` は解答プログラムの出力ファイルのパス、`{test_output}` は対応する `.out` のパスです。
コマンドにこれらのプレースホルダーがなければ、この順序で引数が渡されます。
ジャッジは正解なら終了コード `0`、不正解ならそれ以外を返す必要があります。

```bash
# カスタムジャッジを使って解答をテスト：
cpg test --judge ./judge.rb ./solution.rb

# 直接コマンドを指定してカスタムジャッジを使う場合：
cpg test --judge "ruby ./judge.rb {test_input} {solution_output} {test_output}" ./solution.rb
```

カスタムジャッジを使わず、`.out` がない場合は出力を比較しません。解答プログラムの終了コードが `0` なら `AC`、それ以外は `RE` になります。
空の `.out` が存在する場合は、出力も空である必要があります。

テストディレクトリの代わりに1つの入力ファイルを使うには、`--input-path`（`-I`）を指定してください。
パスはカレントディレクトリからの相対パスで、対応する `.out` があれば想定出力として使います。
`-I -` を指定すると、標準入力から EOF までを1つの入力として読み込みます。この場合は想定出力を使いません。

```bash
cpg test -I ./example.in ./solution.cpp
printf '1 2\n' | cpg test -I - ./solution.cpp
```

`-I -` と `--interactive`（`-i`）を併用すると、解答プログラムと手動でやり取りできます。
出力はすぐに表示され、`--judge` の指定は無視されます。

```bash
cpg test -I - -i ./solution.cpp
```

`--show-io` で、各ケースの入力、想定出力（ある場合）、実際の出力を表示するタイミングを選べます。

- `always`（`a`、想定出力がない場合のデフォルト）: すべてのケースの入出力を表示します。
- `failure`（`f`、デフォルト）: `WA`、`RE`、`TLE`、`MLE` など、失敗したケースの入出力だけを表示します。
- `never`（`n`）: 入出力の詳細を表示しません。

判定結果と集計は常に表示します。解答プログラムとジャッジの標準エラー出力も、そのまま逐次表示します。

```bash
cpg test --show-io always ./solution.cpp
cpg test --show-io never -- ruby ./solution.rb
```

`--panes`（`-p`）で入出力の配置を選べます。

- `none`（`1`、デフォルト）: 入力、想定出力、実際の出力を縦に並べます。
- `outputs`（`2`）: 入力を上に表示し、その下に想定出力と実際の出力を横に並べます。
- `all`（`3`）: 入力、想定出力、実際の出力を横に並べます。

`--query-numbers`（`-n`）を追加すると、想定出力を基準に行番号を付けます。

```bash
cpg test -p outputs -n ./solution.cpp
cpg test -p all -n --show-io always ./solution.cpp
```

入力と想定出力は下揃え、想定出力と実際の出力は上揃えで表示されます。行番号は想定出力を基準に付けられます。

```text
  | Input:    | Expected output: | Actual output:
  | 2         :                  :
  | condition :                  :
1 | query A   | answer A         | answer A
2 | query B   | answer B         | wrong
3 :           :                  | extra
```

`--highlight line` または `--highlight word`（`-H`）で差分を強調表示できます。想定出力側は緑、実際の出力側は赤で表示します。
`line` は異なる行全体を強調します。`word` は対応する行を空白区切りの単語に分け、同じ位置の単語を比較します。
片側だけにある単語や行は、その側で強調します。入力は強調しません。

```bash
cpg test -H line ./solution.cpp
cpg test -p all -n -H word ./solution.cpp
```

出力の末尾の空白を無視するには `--strip` を使います。
`--strip-trailing-newline`（`-S`）では、想定出力と実際の出力の末尾にある CR と LF だけを無視します。スペース、タブ、途中の改行はそのまま比較します。

```bash
cpg test --strip ./solution.cpp

cpg test --strip-trailing-newline ./solution.cpp
```

`--ignore-line-ending` を指定すると、想定出力と実際の出力の CRLF と LF の違いを無視します。
デフォルトで有効ですが、`--no-ignore-line-ending` で無効にできます。

```bash
# CRLF と LF の違いを無視して解答をテスト：
cpg test --ignore-line-ending ./solution.cpp
```

<a id="test-interactive-problems"></a>

## インタラクティブ問題のテスト

<Asciinema src="/competitive-programming-cli/demo/interactive.cast" poster="npt:20" />

カスタムジャッジでインタラクティブ問題をテストできます。
解答プログラムの出力をジャッジの標準入力へ、ジャッジの標準出力を解答プログラムの標準入力へ渡します。
cpg はジャッジの出力に `<`、解答プログラムの出力に `>` を付けます。
やり取りは各ケースの終了後に `--show-io` の設定に従って表示されます。
テストファイルがある場合は、`{test_input}` と `{test_output}` の引数でパスをジャッジに渡し、各ケースでジャッジと解答プログラムを実行します。
カスタムジャッジと同様に、`.out` がなければ、その実行用の空の一時ファイルを代わりに使います。
ほかのテストコマンドと異なり、テストファイルがなくても実行できます。

```bash
# インタラクティブ問題をカスタムジャッジでテスト：
cpg test --interactive --judge ./judge.rb ./solution.rb
```

`--query-numbers`（`-n`）を指定すると、ジャッジの最初の出力に `0` が付けられます。
解答プログラムの最初の出力とジャッジの返答には `1`、次のやり取りには `2` のように番号が付けられます。

```text
0 < initial data
1 > first query
1 < first reply
2 > second query
2 < second reply
```

```bash
cpg test --interactive -n --judge ./judge.rb ./solution.rb
```

`--panes outputs`（`-p 2`）を指定すると、ジャッジを左、解答プログラムを右に表示します。
各発言は時系列順に別々の行へ表示します。中央の矢印はジャッジから解答プログラムへの出力なら `>`、逆方向なら `<` です。

```text
Judge:          Solution:
1 10          >
              < ? 5
HIGHER        >
```

`--query-numbers`（`-n`）を指定すると、番号を中央に表示します。
`|` は発言した側、`:` は空の側を示します。

```text
Judge:              Solution:
1 10          | 0 :
              : 1 | ? 5
HIGHER        | 1 :
              : 2 | ? 8
LOWER         | 2 :
```
