# kindle-capture

Kindle Web ReaderまたはKindle macOSアプリの表示をページ画像として保存し、PDF化するRust製CLIです。
[tsunoda-s-ft/kindle-app](https://github.com/tsunoda-s-ft/kindle-app) の公開されている動作を参考に、コードは流用せずRustで組み直しています。

## 実装方針

- `kindle-capture` だけでキャプチャ、PDF生成、トリミング、重複確認を行う
- macOSアプリの操作にはOS標準の `osascript` と `screencapture` を使う
- Web Readerは通常のChromeを利用者が操作してログインし、その既存セッションへChrome DevTools Protocolで接続する
- 通常利用するChromeとは別のプロファイルを使い、Cookieなどのブラウザデータを分離する
- 既存画像の上書き時は以前のキャプチャを `.capture-backup/` へ退避する
- 重複画像は即削除せず、明示的に適用した場合も `.dedupe-trash/` へ移動する

Web Readerでは、利用できる場合は非公開の`KindleRenderer` APIを使います。現在のReaderのようにAPIが初期化されない場合は、画面上の位置表示と左右キーへ自動的に切り替えます。最初のキー操作で本の進行方向を判定するため、縦書き・横書きの違いを固定値で扱いません。Amazon側の画面変更に強いのはmacOSアプリを使う`app`です。

## 前提条件

- macOS 14以降
- Rust 1.88以降
- Google Chrome（`web` 使用時）
- Kindle macOSアプリ（`app` 使用時）
- 画面収録とアクセシビリティの許可（`app` 使用時）
- 自分のKindleライブラリで閲覧できる書籍

## ビルド

```console
cargo build --release
./target/release/kindle-capture --help
```

## macOSアプリから取得する

Kindleアプリで対象書籍を開いてから実行します。

```console
kindle-capture app --book "My Book" --max-pages 20
```

ウィンドウ検出が合わない場合は領域を指定できます。

```console
kindle-capture app \
  --book "My Book" \
  --region 120,90,1800,2400 \
  --scale 2.0
```

ページ送り後も同じ画像が続くと、既定では5回で終了します。同一判定の画像は最終出力へ採用しないため、末尾の重複を後処理する必要は通常ありません。
描画の揺れで同一判定が働かない場合に備え、`--max-pages` 未指定時も2000ページで停止します。

## Web Readerから取得する

初回実行では専用プロファイルの通常のChromeが開きます。その画面でKindleへログインし、ログイン側の確認画面が出た場合もブラウザ上で完了してください。ツールはログイン操作を自動化せず、取得時に新しいReaderタブを1枚開いて待機します。Reader内の「前回読んでいたページ」確認は現在位置を維持する側で閉じます。次回以降は同じプロファイルとデバッグポートを再利用します。

```console
kindle-capture web --asin B0DSKPTJM5
```

Chromeの起動には`--remote-debugging-port`と`--user-data-dir`だけを使い、`--enable-automation`などの自動操作用フラグは付けません。既定のデバッグポートは`9445`です。ほかの用途と重なる場合は変更できます。

```console
kindle-capture web --asin B0DSKPTJM5 --debug-port 9555
```

キャプチャが終わってもChromeは閉じません。次の書籍でも同じウィンドウを使えます。

`--start`を省略すると、位置表示とページ送りキーを使って先頭まで戻してから取得します。非公開APIを使えない本では、末尾で位置が変わらない場合に3回までページ送りを再試行して終了します。

既定のプロファイルは次の場所です。

```text
~/Library/Application Support/kindle-app-rs/chrome-profile
```

Chrome 136以降は通常のChromeデータディレクトリに対するリモートデバッグが制限されています。そのため、このツールは `~/Library/Application Support/Google/Chrome` の指定を拒否します。背景は[Chrome公式の案内](https://developer.chrome.com/blog/remote-debugging-port)を参照してください。

部分取得やページ上限も指定できます。

```console
kindle-capture web \
  --asin B0DSKPTJM5 \
  --start 500 \
  --end 10000 \
  --max-pages 20
```

## PDF生成

```console
kindle-capture pdf --input ./kindle-captures/B0DSKPTJM5
```

リサイズ時だけJPEGへ変換し、`--quality` を適用します。`--resize 1.0` ではPNGをそのままPDFへ埋め込みます。

```console
kindle-capture pdf \
  --input ./kindle-captures/B0DSKPTJM5 \
  --output my-book.pdf \
  --resize 0.7 \
  --quality 80
```

既存PDFを置き換える場合は `--overwrite` が必要です。

## トリミング

元画像を残したまま、既定では入力ディレクトリ内の `trimmed/` へ出力します。

```console
kindle-capture trim \
  --input ./kindle-captures/MyBook \
  --crop 100,50,1700,2300
```

特定ページだけ処理する場合:

```console
kindle-capture trim \
  --input ./kindle-captures/MyBook \
  --crop 100,50,1700,2300 \
  --pages 5,8,10
```

出力先が空でなければ停止します。既存のトリミング結果を消して続行する動作はありません。

## 末尾の重複確認

既定は確認だけで、ファイルを移動しません。

```console
kindle-capture dedupe-tail --input ./kindle-captures/MyBook
```

結果を反映する場合:

```console
kindle-capture dedupe-tail \
  --input ./kindle-captures/MyBook \
  --apply
```

対象画像は `<input>/.dedupe-trash/` へ移動するため、必要なら戻せます。

## 設定

`config.yaml` は元ツールと同じトップレベル構成です。指定がない項目には既定値を使います。

```console
kindle-capture --config ./config.yaml web --asin B0DSKPTJM5
```

キャプチャ結果は従来どおり `page_0001.png`、`page_0002.png`、`metadata.json` の構成です。Webキャプチャの `metadata.json` も元ツールの項目名を維持しています。

## 検証

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

自動テストでは、入力検証、設定の既定値、画像類似判定、トリミング、復元可能な重複処理、PDFページ数、CLIの終了コードを確認します。Kindleへのログイン、画面収録許可、購入済み書籍が必要な往復操作は手動確認が必要です。

元リポジトリにある書名のオンライン取得、AI向けSkill、切り抜き位置のマーカー画像生成は初版に含めていません。表紙だけ必要な場合は `app --max-pages 1` で1ページ取得できます。

## 注意事項

このツールは私的利用を想定しています。著作権、AmazonおよびKindleの利用条件を守り、生成した画像やPDFを再配布しないでください。固定レイアウト書籍やコミックには対応していません。
