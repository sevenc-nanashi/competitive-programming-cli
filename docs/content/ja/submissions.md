# 提出と結果の確認

<a id="submit-solution"></a>

## 解答の提出

`submit` コマンドで問題に解答を提出できます。

```bash
# 問題に解答を提出：
cpg submit ./solution.cpp

# 短縮形：
cpg s ./solution.cpp

# 提出後に問題のページをブラウザーで開く：
cpg submit ./solution.cpp --open
```

提出先の問題は解答ファイルのディレクトリから親ディレクトリをたどって最初に見つかる `.cpg.toml` で判定します。
`--problem` で問題の URL を指定することもできます。

```bash
# 問題の URL を指定して解答を提出：
cpg submit ./solution.cpp --problem https://atcoder.jp/contests/abc473/tasks/abc473_f
```

提出には事前に以下の設定が必要です：

- 提出先のジャッジにログインする。
- ファイル形式ごとに[提出言語 ID](./configuration.md#submission-language-ids)を設定するか、`--language ID` を指定する。

`--profile NAME`（`-P NAME`）を指定すると、言語プロファイルの `preprocess`、`presubmit`、サービスごとの `submit` 設定を適用します。

言語 ID が未設定または無効な場合は、cpg が利用可能な言語を取得して ID を表示します。

`submit` は、テンプレートから変更されていない解答の提出を拒否します。
`--allow-submit-unchanged-solution` でこの制限を解除できます。

```bash
cpg submit ./solution.cpp --allow-submit-unchanged-solution
```

<a id="copy-a-solution-to-the-clipboard"></a>

## 解答をクリップボードにコピー

```bash
cpg submit ./solution.cpp --clipboard
```

設定された `preprocess` と `presubmit` を実行し、結果を[クリップボードのバックエンド](./configuration.md#clipboard)でコピーします。
`--open` を追加すると、コピー後に問題の URL を表示してブラウザーで開きます。

AtCoder の過去問など、CAPTCHA 認証が必要なジャッジで便利です。

> [!TIP]
> Codeforces では [codeforces-clipboard-submission.user.js](https://sevenc7c.com/competitive-programming-cli/codeforces-clipboard-submission.user.js) を使うと、ファイルを選択する代わりにコピーした解答を貼り付けられます。
> リモートサーバーや WSL を使っている場合など、ブラウザーのファイル選択画面から解答ファイルにアクセスしにくい場合に便利です。  
> インストールには [ScriptCat](https://scriptcat.org) や [Tampermonkey](https://www.tampermonkey.net) などのユーザースクリプトマネージャーが必要です。

<a id="list-results-of-submissions"></a>

## 提出結果の一覧

自分の提出結果を一覧表示できます。

```bash
# 提出一覧を表示：
cpg results

# 短縮形：
cpg r

# インタラクティブな UI で提出結果を監視：
cpg results --ui
```

対象の問題やコンテストは、カレントディレクトリから親ディレクトリをたどって最初に見つかる `.cpg.toml` で判定します。

標準出力をパイプやリダイレクトで渡す場合、`results` は TSV 形式で結果を出力します。
