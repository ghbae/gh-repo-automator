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
- オプションで既存 GitHub リポジトリの公開/非公開（可視性）を変更
- GitHub リポジトリの説明を作成時に設定、または既存リポジトリで変更

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
| `-r`, `--repo-name <NAME>` | 作成・削除時に必須 | - | GitHub 上のリポジトリ名およびローカルのプロジェクトフォルダ名。`.` でカレントディレクトリの操作が可能 |
| `-p`, `--project-type <TYPE>` | 作成時に必須 | - | プロジェクト種別 (`rust`、`python`、`agent`)。`local` 指定時にも必須 |
| `--private <BOOL>` | 任意（作成時のみ） | `true` | プライベートリポジトリにするか (`true` / `false`) |
| `-d`, `--delete` | 任意 | - | 指定した GitHub リポジトリを削除（`--repo-name` は必須。プロジェクト種別は不要） |
| `-l`, `--list` | 任意 | - | GitHub リポジトリの一覧を表示（Private / Public を判別。他のオプションは不要） |
| `-v`, `--visibility <VIS>` | 任意 | - | `public`: 既存GitHubリポジトリを公開化。`private`: GitHubリモートがあれば非公開化、なければ非公開GitHubリポジトリを作成してプッシュ（いずれも `--repo-name` は必須）。`local`: 指定名のローカルプロジェクト・Gitリポジトリを作成 |
| `--description <TEXT>` | 任意 | - | GitHubリポジトリの説明を設定・変更（`--repo-name` は必須）。空文字で既存の説明を削除 |
| `--completion <SHELL>` | 任意 | - | シェル補完スクリプトを出力（`bash` / `zsh`） |
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
gh-repo-automator --delete --repo-name my-rust-app
# または OWNER を明示
gh-repo-automator --delete --repo-name OWNER/REPO
```

`OWNER/` を省略した場合は、認証中（`gh` ログイン中）の GitHub ユーザーのリポジトリとして自動補完されます。削除前に GitHub CLI の確認プロンプトが表示されます（**リポジトリ名を `OWNER/REPO` の完全一致で入力**する必要があります）。削除実行後はリポジトリが実際に消えたことを `gh repo view` で検証し、まだ存在する場合はエラーを報告します。GitHub 上のリポジトリのみを削除し、同名のローカルディレクトリはそのまま残します。

トークンに削除権限（`delete_repo` スコープ）がない場合は、自動的に `gh auth refresh -h github.com -s delete_repo` を起動してスコープを追加してから削除を実行します。

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

#### ローカルリポジトリのGitHubへのアップロードと公開設定

```bash
# 公開 (Public) に変更
gh-repo-automator -r my-rust-app -v public

# GitHubリモートのないローカルGitリポジトリを、非公開GitHubリポジトリとして作成してプッシュ
gh-repo-automator -r my-private-app -v private

# 既にGitHubリモートがある場合は、既存GitHubリポジトリを非公開 (Private) に変更
gh-repo-automator -r OWNER/my-rust-app -v private
```

`-v private` はカレントディレクトリがローカルGitリポジトリであることを確認します。`origin` がGitHubを指していれば既存リポジトリを非公開化し、`origin` がなければ `gh repo create --private --source=. --remote=origin --push` でリポジトリを作成・プッシュします。`origin` がGitHub以外を指す場合は既存リモートを変更せずエラーになります。`OWNER/` を省略した場合は、認証中の GitHub ユーザーのリポジトリとして自動補完されます。なお、公開リポジトリを非公開に変更すると、スター数とウォッチャーは失われます。

#### GitHubリポジトリの説明を設定・変更

```bash
# 既存のGitHubリポジトリの説明を変更
gh-repo-automator -r OWNER/REPO --description "リポジトリの説明"

# カレントリポジトリのoriginを対象に変更（空文字で説明を削除）
gh-repo-automator -r . --description ""

# GitHubリポジトリの作成時に設定
gh-repo-automator -r my-app -p python --description "Pythonアプリ"

# ローカルリポジトリのアップロード、または可視性変更と同時に設定
gh-repo-automator -r . -v private --description "非公開プロジェクト"
```

`--description` は GitHub の About に表示される説明を変更します。`-r .` で既存の説明を変更する場合は、GitHubを指す `origin` が必要です。`-v public` / `-v private` と併用した場合は可視性と説明を同時に変更します（`-v private` で `origin` がない場合は、作成時に説明を設定します）。`--list`、`--delete`、`-v local` とは併用できません。オプション名と同じ文字列や `--` で始まる説明は `--description=<TEXT>` の形式で指定してください。

#### ローカルプロジェクト・Gitリポジトリの作成

```bash
gh-repo-automator --repo-name my-local-app --project-type rust --visibility local
```

`--repo-name` で指定した名前のフォルダをカレントディレクトリ配下に作成し、`--project-type` に応じたテンプレートを配置します。そのフォルダ内で `git init` を実行し、ブランチ名を `main` に設定した後、全ファイルを追加して `feat: initial commit` の初期コミットを作成します。GitHub CLI の認証、GitHub リポジトリの作成、リモートへのプッシュは行いません。

作成後、フォルダへ移動してGitHubへ非公開リポジトリとしてアップロードできます。

```bash
cd my-local-app
gh-repo-automator --repo-name . --visibility private
```

#### カレントディレクトリ・リポジトリを指定する

`-r .` は次の操作に使用できます。

```bash
# カレントディレクトリにテンプレートを配置してGitリポジトリを初期化
gh-repo-automator -r . -p python -v local

# 現在のローカルGitリポジトリを非公開でアップロード、
# または既存のGitHubリポジトリを非公開化
gh-repo-automator -r . -v private

# origin が指すGitHubリポジトリを公開化、または削除
gh-repo-automator -r . -v public
gh-repo-automator -r . --delete
```

`-v local` はカレントディレクトリ名をプロジェクト名として使用します。既存のGitリポジトリ内では実行できず、テンプレートの生成先が既に存在する場合は上書きせずに中止します。それ以外の既存ファイルは初期コミットに含まれます。実行したシェルのカレントディレクトリは変更されません。

Gitリポジトリのサブディレクトリでも既存リポジトリの操作が可能です。`origin` がある場合、GitHub上の対象はそのURLの `OWNER/REPO` から決定します。`-v private` で `origin` がない場合は、Gitリポジトリのルートフォルダ名を使い、認証中ユーザーの非公開リポジトリを作成してコミット済みの内容をプッシュします。未コミットの変更はプッシュされません。`public` と `--delete` にはGitHubの `origin` が必要です。GitHub以外または判定できないURLはエラーになります。

#### TAB 補完の有効化

シェル設定ファイルに以下を追加すると、オプション名とリポジトリ名を TAB で補完できます。

```bash
# bash の場合 (~/.bashrc)
source <(gh-repo-automator --completion bash)

# zsh の場合 (~/.zshrc)
source <(gh-repo-automator --completion zsh)
```

`-r` / `--repo-name` の後では、`.` と `gh repo list` から取得した自分のリポジトリ名が補完されます。`-p`（`rust` / `python` / `agent`）、`-v`（`public` / `private` / `local`）、`--private`（`true` / `false`）の値も補完対象です。

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
       ├─ Rust: cargo init --vcs none --name <repo_name> + .gitignore + README.md
       ├─ Python: pyproject.toml, package/__init__.py, package/main.py, .gitignore, README.md
       └─ Agent: .github/agents/, .github/skills/, .github/loops/, README.md
       │
       ▼
[3. ローカル Git 初期化]
       ├─ git init
       ├─ git branch -m main (ブランチ名を明示的に main 化)
       ├─ git add .
       └─ git commit --allow-empty -m "feat: initial commit"
       │
       ▼
[4. リモート作成 & プッシュ] ─── gh repo create <repo_name> [--private|--public] --source=. --remote=origin --push

```

---

## 主要関数の責務

* `check_or_run_gh_auth()`: `gh auth status` を実行。ステータスが非0の場合は `gh auth login` を起動し、端末の標準入出力を渡してOAuth認証を対話的に完了させる。
* `Config::parse()`: `std::env::args` を順次読み取り、バリデーションと構造体マッピングを実施。
* `init_rust_project(dir, repo_name)`: 指定ディレクトリで `cargo init --vcs none` を実行し、README と `.gitignore` を作成。
* `init_python_project(dir, repo_name)`: `pyproject.toml` やパッケージモジュール等を直接ファイル書き込みで構成。
* `init_agent_project(dir, repo_name)`: `.github/agents/`、`.github/skills/`、`.github/loops/` に編集用サンプルを作成。
* `setup_local_git(dir)`: `git init` 後、`git branch -m main` を実行してブランチ名を `main` に統一した上で初期コミットを作成。
* `create_and_push_github(dir, repo_name, is_private, description)`: `gh repo create` の `--source` と `--push` フラグを組み合わせてリモート反映を一括実行。説明が指定された場合は作成時に設定。
* `list_github_repos()`: `gh repo list --json nameWithOwner,visibility` で一覧を取得し、Private / Public / Internal を判別して整形表示。
* `github_origin_url(dir)`: カレントディレクトリがGitリポジトリであることを確認し、`origin` がGitHubを指す場合はそのURLを返す。`origin` がなければ `None` を返し、GitHub以外を指す場合は安全のためエラーにする。
* `change_repo_settings(repo_name, visibility, description)`: `gh repo edit` で既存リポジトリの可視性や説明を変更。`OWNER` 省略時は `current_gh_login()`（`gh api user`）で認証中ユーザー名を補完。
* `ensure_delete_repo_scope()`: `gh auth status` の `Token scopes:` を確認し、`delete_repo` スコープがなければ `gh auth refresh -h github.com -s delete_repo` を対話実行して追加。
* `delete_github_repo(repo_name)`: `gh repo delete` でリポジトリを削除。`OWNER` 省略時は `current_gh_login()` で認証中ユーザー名を補完し、解決済みの `OWNER/REPO` を返す。
* `verify_repo_deleted(full_name)`: 削除後に `gh repo view` を実行し、リポジトリが実際に削除されたことを検証。まだ存在する場合はエラーを返す。

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
