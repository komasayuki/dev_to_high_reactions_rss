# dev_to_high_reactions_rss

DEV（Forem）の Top 記事（直近 N 日）を reactions 数でフィルタし、Atom フィードとして GitHub Pages に公開する Rust 実装です。

## フィード URL 例
- Project Pages: `https://<owner>.github.io/<repo>/feed.xml`
- User/Org Pages（リポジトリ名が `<owner>.github.io` の場合）: `https://<owner>.github.io/feed.xml`

## 初回セットアップ
1. `gh-pages` ブランチを作成
   - `./scripts/bootstrap_gh_pages.sh` を 1 回実行
2. GitHub Pages の公開元を `gh-pages` ブランチ `/`（root）に設定
   - https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
3. Actions の `contents: write` 権限を確認

## 変更方法
- しきい値（reactions）
  - `config/config.yaml` の `min_reactions` を変更
  - 環境変数 `MIN_REACTIONS` で上書き可能
  - GitHub Variables の `MIN_REACTIONS` を設定すると Actions から優先反映
- 収集期間（lookback）
  - `config/config.yaml` の `lookback_days` を変更
  - 環境変数 `LOOKBACK_DAYS` で上書き可能
- 公開 URL（site_url）
  - `config/config.yaml` の `site_url` を変更
  - 環境変数 `SITE_URL` で上書き可能（未設定なら `GITHUB_REPOSITORY` から自動導出）
- Cron
  - `.github/workflows/update-feed.yml` の `cron` 1 行を書き換えるだけで変更可能

## ローカル実行例
```
cargo run --release -- --config config/config.yaml \
  --state ./public/state/articles.json \
  --out ./public/feed.xml \
  --index ./public/index.html \
  --last-build ./public/last_build.txt
```

## トラブルシュート
- schedule は **UTC** で動作し、混雑時に遅延・ドロップすることがあります。
  - https://docs.github.com/en/actions/learn-github-actions/events-that-trigger-workflows#schedule
  - https://docs.github.com/actions/managing-workflow-runs/disabling-and-enabling-a-workflow
- `gh-pages` ブランチが存在しない / Pages が無効 / 公開元が誤っていると公開されません。

## データソース / 帰属
- Forem API
  - https://developers.forem.com/api/v1
  - https://developers.forem.com/api/v0

## ライセンス
MIT License
