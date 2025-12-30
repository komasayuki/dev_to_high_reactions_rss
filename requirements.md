# requirements.md（Codex CLI 用・Rust 実装）
## 目的
GitHub リポジトリ一式を作成する。内容は以下。

- DEV（Forem）の「Top（直近 N 日の人気記事）」を定期収集
- **reactions 数のしきい値（デフォルト: 30 以上）**でフィルタ
- **Atom 1.0 形式（feed.xml）**を生成
- **GitHub Actions を 1 時間周期**で実行し、成果物を **GitHub Pages**（`gh-pages` ブランチ）で公開

実装言語は **Rust** とする（Python 実装は禁止）。
しきい値と収集周期（cron）は **変更しやすい**構成にする。

## データソース（HTML スクレイピング禁止）
`https://dev.to/top/week` の HTML をスクレイピングしないこと。
代わりに Forem/DEV の API を使用する。

- API: `GET https://dev.to/api/articles`
- 主に使用するクエリ:
  - `top=<LOOKBACK_DAYS>`（直近 N 日の人気記事）
  - `per_page=<PER_PAGE>`
  - `page=<PAGE>`

参照（Forem API ドキュメント）:
- https://developers.forem.com/api/v1
- https://developers.forem.com/api/v0

## 機能要件
### 1) 収集・フィルタ
1. 記事取得:
   - 取得 URL 例: `https://dev.to/api/articles?top=7&per_page=100&page=1`
   - デフォルト:
     - `LOOKBACK_DAYS=7`（「week」相当）
     - `PER_PAGE=100`
     - `MAX_PAGES=5`（複数ページ取得して候補数を確保）
   - HTTP タイムアウト: 10〜20 秒
   - 一時的失敗（5xx/タイムアウト/ネットワーク）に対して **最大 3 回リトライ**（指数バックオフ）
   - 失敗時は原因が分かるエラーメッセージを出す（API URL・HTTP ステータス・リトライ回数など）

2. フィルタ条件:
   - `public_reactions_count >= MIN_REACTIONS` を満たす記事のみ残す
   - `positive_reactions_count` も保存し、将来の切替に備える

3. 重複排除:
   - `id`（数値）で一意に扱う
   - ページ跨ぎ・複数回実行で重複が出ても状態ファイル上で 1 件に統合
   - `id` が欠落する場合のみ `canonical_url` をフォールバックキーとする

### 2) 状態の永続化（実行間で保持）
ワークフロー実行はステートレスになりがちなので、**GitHub Pages 側（`gh-pages` ブランチ）に状態をコミット**して継続実行可能にする。

- 状態ファイル: `state/articles.json`（`gh-pages` ブランチのルート配下に `state/` を作る）
- 保存内容（最低限）:
  - `id` / `canonical_url` / `title`
  - `public_reactions_count` / `positive_reactions_count`
  - 日付系（`published_timestamp` / `edited_at` 等のうち利用できるもの）
  - `last_seen`（今回実行で観測した時刻）

保持ポリシー（毎回 prune する）:
- `MAX_STORED_DAYS`（デフォルト 60 日）を超えたものを削除
- さらに上限として `MAX_STORED_ITEMS`（デフォルト 1000 件）を超えたら古い順に削除

### 3) 生成成果物（GitHub Pages で公開されるもの）
`gh-pages` ブランチ（ルート）に以下を生成・更新してコミットする。

必須:
- `feed.xml`（Atom フィード）
- `index.html`（簡易トップ。`feed.xml` へのリンク、設定値、最終更新時刻を表示）
- `state/articles.json`（永続状態）
- `last_build.txt`（ISO 8601 / RFC3339 形式の最終実行時刻）
- `.nojekyll`（Jekyll による不要な変換を避ける）

注意:
- `last_build.txt` は **毎回必ず更新**する（記事が変わらなくても Pages の更新時刻が追える、また Actions 停止検知にも役立つ）。

### 4) Atom フィード仕様（Atom 1.0）
`feed.xml` は Atom 1.0 として最低限の必須要素を満たすこと。

Feed レベル（必須）:
- ルート要素: `<feed xmlns="http://www.w3.org/2005/Atom">`
- 必須要素: `id`, `title`, `updated`
- `link rel="self"`（`feed.xml` を指す）
- `link rel="alternate"`（サイトの `index.html` を指す）
- `id`: Pages URL から一意に導出（または `tag:` URI）
- `updated`: **最も新しい entry の updated**、entry が 0 件ならビルド時刻（RFC3339）

Entry レベル（必須）:
- `id`, `title`, `link`, `updated`
- `summary` もしくは `content` のいずれか（ここでは `summary type="html"` を必須化）

Entry のフィールド選択:
- `id`: `tag:dev.to,YYYY:<article_id>` または `canonical_url`
- `link`: `canonical_url` を優先、なければ `url`
- `updated`: `edited_at` と `published_timestamp` のうち利用可能な「より新しい」値を採用（両方ない場合は `published_at` 等の代替）
- `summary (type="html")` に含める内容（最低限）:
  - reactions（public / positive）
  - 著者名（可能ならプロフィール URL も）
  - 公開日
  - タグ一覧
  - description があれば含める

並び順:
- reactions 降順 → 公開日降順

フィード掲載件数上限:
- `MAX_FEED_ENTRIES`（デフォルト 200）

### 5) GitHub Pages 公開モデル
`gh-pages` ブランチを Pages の公開元にする（`main` に生成物を混ぜない）。

README に必ず記載すること:
- GitHub Pages の Source を **`gh-pages` ブランチ /（root）**に設定する
- 公開 URL 例:
  - Project Pages: `https://<owner>.github.io/<repo>/feed.xml`
  - 特例（User/Org Pages）: リポジトリ名が `<owner>.github.io` の場合は `https://<owner>.github.io/feed.xml`

参照（GitHub Pages）:
- https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site

### 6) GitHub Actions（1 時間周期）
`.github/workflows/update-feed.yml` を作成する。

トリガ:
- `schedule`: 1 時間ごと（デフォルト: 毎時 7 分 `7 * * * *`。トップ・オブ・アワー集中を避ける意図）
- `workflow_dispatch`: 手動実行

権限:
- `permissions: contents: write`（`gh-pages` へ push するため）

Concurrency:
- 重複実行を避ける（同一グループ、`cancel-in-progress: true`）

ワークフロー手順（必須）:
1. `main` を checkout（コード）: `actions/checkout@v4`
2. `gh-pages` を `public/` に checkout:
   - `actions/checkout@v4` with `ref: gh-pages`, `path: public`, `fetch-depth: 0`
   - `gh-pages` が存在しない場合に備え、**ブートストラップ**を用意する（後述）
3. Rust toolchain セットアップ（例: `dtolnay/rust-toolchain@stable`）
4. Cargo キャッシュ（`~/.cargo/registry`, `~/.cargo/git`, `target/`）
5. ビルド & 実行:
   - `cargo run --release --bin devto-feed -- --config config/config.yaml --state public/state/articles.json --out public/feed.xml --index public/index.html --last-build public/last_build.txt`
6. `gh-pages` 側に変更があれば commit/push（なければ何もしない）
   - `git add -A`
   - `git diff --cached --quiet` で判定
   - commit message 例: `chore(feed): update`

重要な注意（README に必ず記載）:
- `schedule` は **UTC** 基準で動く
- 混雑時に遅延・ドロップすることがある
- デフォルトブランチ上の workflow が対象になる

参照（GitHub Actions schedule）:
- https://docs.github.com/en/actions/learn-github-actions/events-that-trigger-workflows#schedule
- https://docs.github.com/actions/managing-workflow-runs/disabling-and-enabling-a-workflow

### 7) 設定（しきい値と周期を変更しやすく）
しきい値（`MIN_REACTIONS`）と収集周期（cron）は変更しやすい構成にする。

設定ファイル（必須）:
- `config/config.yaml`
- デフォルト値:
  - `min_reactions: 30`
  - `lookback_days: 7`
  - `per_page: 100`
  - `max_pages: 5`
  - `max_feed_entries: 200`
  - `max_stored_days: 60`
  - `max_stored_items: 1000`
  - `site_title: "DEV Top filtered"`
  - `site_description: "DEV Top articles filtered by reactions threshold"`
  - `site_url: ""`（空なら `GITHUB_REPOSITORY` から自動導出）
  - `feed_path: "feed.xml"`

環境変数で上書き（必須）:
- `MIN_REACTIONS`
- `LOOKBACK_DAYS`
- `SITE_URL`

GitHub Actions からの上書き（必須）:
- リポジトリ変数（GitHub Variables）を優先できるようにする（未設定なら空になり得る点に注意）
  - `MIN_REACTIONS: ${{ vars.MIN_REACTIONS }}`
  - `LOOKBACK_DAYS: ${{ vars.LOOKBACK_DAYS }}`
- プログラム側で「空/未設定」を検知して config のデフォルトにフォールバックすること

周期変更:
- `.github/workflows/update-feed.yml` の cron 1 行を書き換えるだけで変更できること

### 8) ブートストラップ（初回セットアップ）
推奨方式（A）として、ユーザが一度だけ実行するスクリプトを用意する。

A. `scripts/bootstrap_gh_pages.sh`（必須）:
- orphan `gh-pages` ブランチ作成
- `.nojekyll`, `index.html`（最小）, `state/articles.json`（空）, `last_build.txt` を配置
- push して `gh-pages` を作成
- 以後 Actions がそのブランチを更新する

README に手順として明記する。

## 非機能要件
### 信頼性
- リトライと明確なエラー出力
- 冪等性（同じ記事が state/feed に重複しない）
- 出力の決定性（並び順がブレない）

### 性能
- 通常時 30 秒以内で完走を目標
- 依存を過剰に増やさない

### セキュリティ/プライバシー
- secrets 不要
- 公開 API の情報以外は保持しない
- GitHub トークンを生成物（`gh-pages`）に書き込まない

### 保守性
- 標準的な Rust クレート構成
- `rustfmt` と `clippy` を CI で実行（推奨）
- エラーコードを仕様化し、CI/運用で判断できるようにする

## Rust 実装要件
### 推奨クレート
（必須ではないが、合理的な選定として推奨。採用する場合はバージョンを `Cargo.toml` で固定する。）
- HTTP: `reqwest`（blocking か async のどちらかに統一）
- JSON: `serde`, `serde_json`
- YAML: `serde_yaml`
- Date: `chrono`（RFC3339）
- CLI: `clap`
- XML: `quick-xml`（Writer 推奨）
- backoff: 自前実装 or `backoff` crate

### データモデル
API レスポンスの必要項目のみを `struct` に定義（serde でデシリアライズ）。最低限:
- `id`, `title`, `url`, `canonical_url`
- `description`（任意）
- `published_timestamp`, `published_at`, `edited_at`（API の実際の型/有無に合わせる）
- `public_reactions_count`, `positive_reactions_count`
- `tag_list` 等（API の形式に合わせる）
- `user`: `name`, `username`（プロフィール URL を組み立て可能に）

状態ファイル（JSON）の内部形式:
- 配列 or `id` キーの map いずれでも可
- prune と dedupe が簡単になる設計にする

## CLI 仕様（必須）
バイナリ名: `devto-feed`

引数（必須）:
- `--config <path>`
- `--state <path>`
- `--out <path>`（feed.xml）
- `--index <path>`（index.html）
- `--last-build <path>`（last_build.txt）
- `--dry-run`（書き込みせず、差分サマリだけ表示）

終了コード（必須）:
- `0`: 成功
- `2`: 設定エラー（YAML パース失敗、値域不正など）
- `3`: ネットワーク/API 失敗（HTTP 失敗、API 形式不整合など）
- `4`: フィード生成失敗（XML 生成/書き込み、必須要素不足など）

## リポジトリ構成（必須）
```
.
├─ .github/
│  └─ workflows/
│     └─ update-feed.yml
├─ config/
│  └─ config.yaml
├─ src/
│  ├─ main.rs            # CLI + オーケストレーション
│  ├─ config.rs          # config 読み込み + env 上書き
│  ├─ devto_api.rs       # 取得 + リトライ
│  ├─ state.rs           # state load/save/prune/dedupe
│  ├─ atom.rs            # Atom 生成
│  └─ html.rs            # index.html 生成
├─ scripts/
│  └─ bootstrap_gh_pages.sh
├─ tests/
│  └─ atom_minimum.rs    # Atom 最低要件検証（XML パース + 必須要素）
├─ Cargo.toml
├─ README.md
└─ LICENSE               # MIT 推奨
```

## README.md 作成要件（必須・Codex が作ること）
README.md に以下を必ず含める。

1. このリポジトリが何をするか（DEV Top/Week を reactions でフィルタして Atom を公開）。
2. フィード URL 例（Project Pages / User Pages の違い）。
3. 初回セットアップ手順:
   - `scripts/bootstrap_gh_pages.sh` を 1 回実行
   - GitHub Pages を `gh-pages` ブランチ `/` に設定
   - Actions の権限（contents write）確認
4. 変更方法:
   - しきい値（config/env/vars）
   - cron（workflow の 1 行）
   - lookback days（config/env）
5. ローカル実行方法（例）:
   - `cargo run --release -- --config config/config.yaml --state ./public/state/articles.json --out ./public/feed.xml --index ./public/index.html --last-build ./public/last_build.txt`
6. トラブルシュート:
   - schedule の遅延/ドロップ、UTC である点（公式ドキュメントリンク）
   - `gh-pages` が無い、Pages が無効、公開元が誤っている
7. データソース/帰属:
   - Forem API ドキュメントリンク
8. ライセンス

## 受け入れ条件（DoD）
- 初回 bootstrap + Pages 有効化後、`feed.xml` が Pages URL で取得できる。
- Actions が 1 時間周期で実行され、`gh-pages` の `last_build.txt` と `feed.xml` が更新される。
- `min_reactions` デフォルト 30 で、コード改修なしに変更できる（config/env/vars）。
- cron は workflow の 1 行で変更できる。
- テストが Atom の最低要件を検証し、CI で通る。
- README に必要事項がすべて含まれる。

## 実装時の注意（必須）
- `https://dev.to/top/week` のスクレイピングは行わず、API の `top=7` を使う。
- schedule は UTC で動き、遅延/ドロップしうる（厳密な毎時実行を保証できない）。
- workflow は default branch 上で定義されている必要がある。
- 生成物は `gh-pages` にのみ commit（`main` を汚さない）。
- `gh-pages` ルートに `.nojekyll` を置く。
