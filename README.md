# gh-repo-automator

ローカル環境にプロジェクト（Rust、Python、またはスキル・エージェント・ループ管理用）の基本テンプレートを生成し、初期コミットの作成から GitHub リモートリポジトリの作成・プッシュまでを一括自動化する CLI ツールです。

---

# 1. ユーザー向けガイド

## 概要

`gh-repo-automator` は、コマンドを1回実行するだけで以下の作業をすべて完了させます。

- **自動認証対応**: GitHub CLI (`gh`) が未認証の場合、その場で自動的に対話型ログイン画面（ブラウザ／ワンタイムコード認証）を起動
- カレントディレクトリ直下に指定したプロジェクト名（リポジトリ名）のディレクトリを作成
- 指定した種別（Rust、Python、またはスキル・エージェント・ループ管理用）の標準的な雛形を配置
- ローカル Git リポジトリを初期化し、デフォルトブランチを `main` に設定して初期コミットを作成
- GitHub 上に指定した名前でリモートリポジトリを作成し、ローカルの `main` ブランチを自動プッシュ
- オプションで GitHub リポジトリを削除（`gh` の確認プロンプトを表示。ローカルのファイルは削除しません）
- オプションで GitHub リポジトリの一覧を表示（Private / Public を判別）

---

## 前提条件

実行環境に以下のツールがインストールされており、パスが通っている必要があります。

1. **Git**
2. **GitHub CLI (`gh`)**（未ログインでもツールの実行時に自動でログインプロンプトが開きます）
3. **Rust / Cargo**（Rust プロジェクトを生成する場合）

---

## インストール方法

### インストーラーで登録する場合（推奨）
リポジトリのルートディレクトリで以下を実行します。

```bash
./install.sh
```

スクリプトが `cargo install --path . --force` を実行し、実行ファイルを `~/.cargo/bin/gh-repo-automator` に配置します。`CARGO_HOME` を設定している場合は、その `bin` ディレクトリがインストール先です。必要に応じて、表示された案内に従って `bin` を `PATH` に追加してください。

インストーラーを使わずに直接登録する場合:

```bash
cargo install --path . --force

```

以降は `gh-repo-automator` コマンドとして実行できます。

### リリースバイナリを直接ビルドする場合

```bash
cargo build --release

```

`./target/release/gh-repo-automator` にバイナリが生成されます。

---

## 使い方

### オプション一覧

| オプション | 必須 | デフォルト値 | 説明 |
| --- | --- | --- | --- |
| `-r`, `--repo-name <NAME>` | 作成・削除時に必須 | - | GitHub 上のリポジトリ名およびローカルのプロジェクトフォルダ名 |
| `-p`, `--project-type <TYPE>` | 作成時に必須 | - | プロジェクト種別 (`rust`、`python`、`agent`) |
| `--private <BOOL>` | 任意（作成時のみ） | `true` | プライベートリポジトリにするか (`true` / `false`) |
| `-d`, `--delete` | 任意 | - | 指定した GitHub リポジトリを削除（`--repo-name` は必須。プロジェクト種別は不要） |
| `-l`, `--list` | 任意 | - | GitHub リポジトリの一覧を表示（Private / Public を判別。他のオプションは不要） |
| `-h`, `--help` | 任意 | - | ヘルプメッセージの表示 |

### 実行例

#### 1. Rust プロジェクトを作成してプッシュ

* **プライベートリポジトリ（デフォルト）:**
```bash
gh-repo-automator -r my-rust-app -p rust

```


* **パブリックリポジトリ:**
```bash
gh-repo-automator -r my-rust-app -p rust --private false

```



#### 2. Python プロジェクトを作成してプッシュ

* **プライベートリポジトリ（デフォルト）:**
```bash
gh-repo-automator -r my-python-app -p python

```


* **パブリックリポジトリ:**
```bash
gh-repo-automator -r my-python-app -p python --private false

```



※ 未ログイン状態で実行した場合、ターミナル上で GitHub CLI のログイン選択画面（ブラウザ／認証コード）が表示されます。認証完了後、そのまま後続の処理が継続します。

#### 3. スキル・エージェント・ループ用リポジトリを作成してプッシュ

```bash
gh-repo-automator -r my-agent-docs -p agent
```

`.github/agents/` にエージェント定義、`.github/skills/` にスキル定義、`.github/loops/` に作業ループのサンプルを生成します。

#### GitHub リポジトリの削除

```bash
gh-repo-automator --delete --repo-name OWNER/REPO
```

`OWNER/REPO` の代わりに、現在の GitHub ホストで解決できるリポジトリ名も指定できます。削除前に GitHub CLI の確認プロンプトが表示されます。GitHub 上のリポジトリのみを削除し、同名のローカルディレクトリはそのまま残します。

#### GitHub リポジトリの一覧表示

```bash
gh-repo-automator --list
```

認証中のアカウントのリポジトリを最大 1000 件取得し、各リポジトリの可視性（`Private` / `Public` / `Internal`）を併せて表示します。

```text
REPOSITORY                 VISIBILITY
-------------------------  ----------
your-account/my-rust-app   Private
your-account/my-oss-tool   Public

合計: 2 件
```

---

# 2. 開発者向けガイド

## アーキテクチャと設計方針

* **外部クレートゼロ（Zero External Dependencies）**:
* 引数解析クレート（`clap` 等）や Git クレート（`git2` 等）は使用せず、Rust の標準ライブラリ（`std::env::args`, `std::process::Command`, `std::fs`）のみで完結しています。
* クレート取得が不要なため、オフライン環境でも高速にコンパイル・ビルドが可能です。


* **インタラクティブ認証のシームレスな委譲**:
* `check_or_run_gh_auth()` では `Stdio::inherit()` を指定し、現在の標準入出力を直接 `gh auth login` プロセスに接続することで、Rust CLI 内からターミナルの対話型認証を完結させています。



---

## 内部処理シーケンス

```text
[CLI引数の解析 (Config::parse)]
       │
       ▼
[0. GitHub CLI 認証状態の確認]
       ├─ gh auth status が 0 ─── そのまま次へ
       └─ 未ログイン ───────────── gh auth login を対話実行 (Stdio::inherit)
       │
       ▼
[1. プロジェクトフォルダ作成] ─── ./{repo_name} ディレクトリ作成 (既存時は中断)
       │
       ▼
[2. テンプレート展開]
       ├─ Rust: cargo init --name <repo_name> + README.md
       ├─ Python: pyproject.toml, package/__init__.py, package/main.py, .gitignore, README.md
       └─ Agent: .github/agents/, .github/skills/, .github/loops/, README.md
       │
       ▼
[3. ローカル Git 初期化]
       ├─ git init
       ├─ git branch -m main (ブランチ名を明示的に main 化)
       ├─ git add .
       └─ git commit -m "feat: initial commit"
       │
       ▼
[4. リモート作成 & プッシュ] ─── gh repo create <repo_name> [--private|--public] --source=. --remote=origin --push

```

---

## 主要関数の責務

* `check_or_run_gh_auth()`: `gh auth status` を実行。ステータスが非0の場合は `gh auth login` を起動し、端末の標準入出力を渡してOAuth認証を対話的に完了させる。
* `Config::parse()`: `std::env::args` を順次読み取り、バリデーションと構造体マッピングを実施。
* `init_rust_project(dir, repo_name)`: カレントディレクトリを `./{repo_name}` に切り替えて `cargo init` を発行。
* `init_python_project(dir, repo_name)`: `pyproject.toml` やパッケージモジュール等を直接ファイル書き込みで構成。
* `init_agent_project(dir, repo_name)`: `.github/agents/`、`.github/skills/`、`.github/loops/` に編集用サンプルを作成。
* `setup_local_git(dir)`: `git init` 後、`git branch -m main` を実行してブランチ名を `main` に統一した上で初期コミットを作成。
* `create_and_push_github(dir, repo_name, is_private)`: `gh repo create` の `--source` と `--push` フラグを組み合わせてリモート反映を一括実行。
* `list_github_repos()`: `gh repo list --json nameWithOwner,visibility` で一覧を取得し、Private / Public / Internal を判別して整形表示。

---

## ビルドとテスト

```bash
# フォーマットと構文チェック
cargo check
cargo fmt --check

# リリースビルドの作成
cargo build --release

```

---

## ライセンス

MIT License — 詳細は [LICENSE](./LICENSE) を参照してください。
