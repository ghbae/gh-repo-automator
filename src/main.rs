use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
enum ProjectType {
    Rust,
    Python,
    Agent,
}

#[derive(Debug)]
struct Config {
    repo_name: Option<String>,
    project_type: Option<ProjectType>,
    private: bool,
    delete: bool,
    list: bool,
    visibility: Option<String>,
}

impl Config {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        let mut repo_name = None;
        let mut project_type = None;
        let mut private = true;
        let mut delete = false;
        let mut list = false;
        let mut visibility = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-r" | "--repo-name" => {
                    i += 1;
                    if i < args.len() {
                        repo_name = Some(args[i].clone());
                    } else {
                        return Err("--repo-name の値が指定されていません。".into());
                    }
                }
                "-p" | "--project-type" => {
                    i += 1;
                    if i < args.len() {
                        match args[i].to_lowercase().as_str() {
                            "rust" => project_type = Some(ProjectType::Rust),
                            "python" => project_type = Some(ProjectType::Python),
                            "agent" => project_type = Some(ProjectType::Agent),
                            other => return Err(format!("不正なプロジェクト種別 '{}' です。(rust、python、agent のいずれかを指定してください)", other)),
                        }
                    } else {
                        return Err("--project-type の値が指定されていません。".into());
                    }
                }
                "--private" => {
                    i += 1;
                    if i < args.len() {
                        match args[i].to_lowercase().as_str() {
                            "true" => private = true,
                            "false" => private = false,
                            _ => return Err("--private には true または false を指定してください。".into()),
                        }
                    } else {
                        return Err("--private の値が指定されていません。".into());
                    }
                }
                "-d" | "--delete" => delete = true,
                "-l" | "--list" => list = true,
                "-v" | "--visibility" => {
                    i += 1;
                    if i < args.len() {
                        match args[i].to_lowercase().as_str() {
                            v @ ("public" | "private") => visibility = Some(v.to_string()),
                            other => return Err(format!("不正な可視性 '{}' です。(public または private を指定してください)", other)),
                        }
                    } else {
                        return Err("--visibility の値が指定されていません。".into());
                    }
                }
                "-h" | "--help" => {
                    Self::print_help(&args[0]);
                    std::process::exit(0);
                }
                unknown => return Err(format!("不明なオプションです: {}", unknown)),
            }
            i += 1;
        }

        if !list {
            if repo_name.is_none() {
                return Err("リポジトリ名が指定されていません。(-r <NAME> または --repo-name <NAME>)".into());
            }
            if !delete && visibility.is_none() && project_type.is_none() {
                return Err("プロジェクト種別が指定されていません。(-p <rust|python|agent> または --project-type <rust|python|agent>)".into());
            }
        }

        Ok(Config {
            repo_name,
            project_type,
            private,
            delete,
            list,
            visibility,
        })
    }

    fn print_help(prog_name: &str) {
        println!("使い方: {} [オプション]", prog_name);
        println!();
        println!("オプション:");
        println!("  -r, --repo-name <NAME>      GitHubリポジトリ / プロジェクト名 (必須)");
        println!(
            "  -p, --project-type <TYPE>   プロジェクト種別: rust / python / agent (作成時に必須)"
        );
        println!(
            "      --private <BOOL>        プライベートリポジトリにするか (作成時のデフォルト: true)"
        );
        println!(
            "  -d, --delete                GitHubリポジトリを削除する (プロジェクト種別は不要)"
        );
        println!(
            "  -l, --list                  GitHubリポジトリの一覧を表示する (Private/Public判別付き)"
        );
        println!(
            "  -v, --visibility <VIS>      既存リポジトリの可視性を変更する: public / private (--repo-name と併用)"
        );
        println!("  -h, --help                  ヘルプの表示");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match Config::parse() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("エラー: {}", err);
            eprintln!("ヘルプは --help を付けて実行してください。");
            std::process::exit(1);
        }
    };

    // 0. GitHub CLI (gh) の認証状態を確認（未認証ならその場でログイン実行）
    println!("0. GitHub CLI (gh) の認証確認中...");
    check_or_run_gh_auth()?;

    if config.list {
        println!("GitHubリポジトリの一覧を取得します...");
        list_github_repos()?;
        return Ok(());
    }

    if config.delete {
        let repo_name = config.repo_name.as_deref().expect("repo name is required when deleting");
        println!("リポジトリ削除権限 (delete_repo スコープ) を確認中...");
        ensure_delete_repo_scope()?;
        println!(
            "GitHubリポジトリ '{}' の削除を開始します...",
            repo_name
        );
        delete_github_repo(repo_name)?;
        println!("GitHubリポジトリ '{}' を削除しました。", repo_name);
        return Ok(());
    }

    if let Some(visibility) = &config.visibility {
        let repo_name = config
            .repo_name
            .as_deref()
            .expect("repo name is required when changing visibility");
        println!(
            "GitHubリポジトリ '{}' の可視性を '{}' に変更します...",
            repo_name, visibility
        );
        change_repo_visibility(repo_name, visibility)?;
        println!(
            "GitHubリポジトリ '{}' を {} に変更しました。",
            repo_name,
            if visibility == "private" { "非公開 (Private)" } else { "公開 (Public)" }
        );
        return Ok(());
    }

    let repo_name = config.repo_name.as_deref().expect("repo name is required when creating");

    // 1. プロジェクトディレクトリの作成
    let local_dir = PathBuf::from(repo_name);
    if local_dir.exists() {
        return Err(format!("カレントディレクトリに '{}' ディレクトリが既に存在します。", repo_name).into());
    }
    fs::create_dir_all(&local_dir)?;
    println!("1. プロジェクトディレクトリを作成しました: {}", local_dir.display());

    // 2. テンプレート展開
    match config.project_type {
        Some(ProjectType::Rust) => {
            println!("2. Rustプロジェクトを初期化中 (cargo init)...");
            init_rust_project(&local_dir, repo_name)?;
        }
        Some(ProjectType::Python) => {
            println!("2. Pythonプロジェクトを初期化中...");
            init_python_project(&local_dir, repo_name)?;
        }
        Some(ProjectType::Agent) => {
            println!("2. スキル・エージェント・ループ用のひな形を作成中...");
            init_agent_project(&local_dir, repo_name)?;
        }
        None => unreachable!("project type is required when creating a repository"),
    }

    // 3. ローカルGit初期化・ブランチ名 main 化・初期コミット
    println!("3. ローカルGitの初期化と初期コミット (mainブランチ)...");
    setup_local_git(&local_dir)?;

    // 4. GitHubリポジトリ作成・プッシュ
    println!("4. GitHubリポジトリ '{}' を作成してプッシュ中...", repo_name);
    create_and_push_github(&local_dir, repo_name, config.private)?;

    println!("\nすべて完了しました。");
    println!("- ローカル: {}", local_dir.canonicalize()?.display());
    println!("- リモート: https://github.com/<your-account>/{}", repo_name);

    Ok(())
}

/// gh auth status をチェックし、未認証であれば対話型 gh auth login を起動する
fn check_or_run_gh_auth() -> Result<(), Box<dyn std::error::Error>> {
    let check = Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match check {
        Ok(status) if status.success() => {
            println!("   -> GitHub認証確認済みです。");
            Ok(())
        }
        Ok(_) => {
            println!("   -> GitHub CLI 未ログインです。ログイン処理 (gh auth login) を起動します...");
            // 対話型CLIを実行するためにターミナル入出力を継承
            let login_status = Command::new("gh")
                .arg("auth")
                .arg("login")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()?;

            if login_status.success() {
                println!("   -> ログインが完了しました。処理を続行します。");
                Ok(())
            } else {
                Err("GitHub CLI のログインが完了しませんでした。".into())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("GitHub CLI (`gh`) が見つかりません。事前にインストールしてください。".into())
        }
        Err(e) => Err(format!("gh コマンドの実行に失敗しました: {}", e).into()),
    }
}

fn init_rust_project(dir: &Path, repo_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .current_dir(dir)
        .args(["init", "--name", repo_name])
        .status()?;

    if !status.success() {
        return Err("cargo init に失敗しました。".into());
    }

    OpenOptions::new()
        .append(true)
        .open(dir.join(".gitignore"))?
        .write_all(b"Cargo.lock\n")?;

    let readme = format!("# {}\n\nA Rust project generated by gh-repo-automator.\n", repo_name);
    File::create(dir.join("README.md"))?.write_all(readme.as_bytes())?;

    Ok(())
}

fn init_python_project(dir: &Path, repo_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pkg_name = repo_name.replace('-', "_");
    let pkg_dir = dir.join(&pkg_name);
    fs::create_dir_all(&pkg_dir)?;

    let pyproject = format!(
        r#"[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[project]
name = "{}"
version = "0.1.0"
description = "Python project"
readme = "README.md"
requires-python = ">=3.10"
dependencies = []
"#,
        repo_name
    );
    File::create(dir.join("pyproject.toml"))?.write_all(pyproject.as_bytes())?;

    File::create(pkg_dir.join("__init__.py"))?.write_all(b"")?;
    let main_py = "def main():\n    print(\"Hello from generated Python project!\")\n\nif __name__ == \"__main__\":\n    main()\n";
    File::create(pkg_dir.join("main.py"))?.write_all(main_py.as_bytes())?;

    let gitignore = "__pycache__/\n*.py[cod]\n*$py.class\n.venv/\nvenv/\ndist/\nbuild/\n*.egg-info/\n";
    File::create(dir.join(".gitignore"))?.write_all(gitignore.as_bytes())?;

    let readme = format!("# {}\n\nA Python project generated by gh-repo-automator.\n", repo_name);
    File::create(dir.join("README.md"))?.write_all(readme.as_bytes())?;

    Ok(())
}

fn init_agent_project(dir: &Path, repo_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let agents_dir = dir.join(".github/agents");
    let skill_dir = dir.join(".github/skills/example-skill");
    let loops_dir = dir.join(".github/loops");
    fs::create_dir_all(&agents_dir)?;
    fs::create_dir_all(&skill_dir)?;
    fs::create_dir_all(&loops_dir)?;

    let readme = format!(
        r#"# {repo_name}

スキル、エージェント、作業ループを管理するリポジトリです。

## ディレクトリ構成

- `.github/agents/`: エージェント定義 (`*.agent.md`)
- `.github/skills/`: スキル定義 (`<skill-name>/SKILL.md`)
- `.github/loops/`: 繰り返し実行する作業手順・ループの定義

各サンプルを編集し、用途に合わせて追加してください。
"#
    );
    File::create(dir.join("README.md"))?.write_all(readme.as_bytes())?;

    let agent = r#"---
name: example-agent
description: このエージェントを使う場面を説明してください。
---

# Example agent

このエージェントが担当する役割と、守るべき指示を記載してください。
"#;
    File::create(agents_dir.join("example.agent.md"))?.write_all(agent.as_bytes())?;

    let skill = r#"---
name: example-skill
description: このスキルを使う作業と適用条件を説明してください。
---

# Example skill

このスキルを実行するための手順や判断基準を記載してください。
"#;
    File::create(skill_dir.join("SKILL.md"))?.write_all(skill.as_bytes())?;

    let loop_definition = r#"# Example loop

## Goal

このループで達成する目標を記載してください。

## Steps

1. 現在の状態を確認する。
2. 目標に近づくための作業を1つ実行する。
3. 結果を検証し、未達なら次の反復に進む。

## Stop condition

完了と判断する条件、または継続を中止する条件を記載してください。
"#;
    File::create(loops_dir.join("example.md"))?.write_all(loop_definition.as_bytes())?;

    Ok(())
}

fn setup_local_git(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_cmd(dir, "git", &["init"])?;
    run_cmd(dir, "git", &["branch", "-m", "main"])?;
    run_cmd(dir, "git", &["add", "."])?;
    run_cmd(dir, "git", &["commit", "-m", "feat: initial commit"])?;
    Ok(())
}

fn create_and_push_github(
    dir: &Path,
    repo_name: &str,
    is_private: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let visibility_flag = if is_private { "--private" } else { "--public" };

    run_cmd(
        dir,
        "gh",
        &[
            "repo",
            "create",
            repo_name,
            visibility_flag,
            "--source=.",
            "--remote=origin",
            "--push",
        ],
    )?;

    Ok(())
}

fn delete_github_repo(repo_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // OWNER 省略時は認証中ユーザーのリポジトリとして補完する
    let full_name = if repo_name.contains('/') {
        repo_name.to_string()
    } else {
        let owner = current_gh_login()?;
        let resolved = format!("{}/{}", owner, repo_name);
        println!("   -> 削除対象を '{}' に解決しました。", resolved);
        resolved
    };

    let status = Command::new("gh")
        .args(["repo", "delete", &full_name])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(format!("GitHubリポジトリ '{}' の削除に失敗しました。", full_name).into());
    }
    Ok(())
}

/// トークンに delete_repo スコープがあるか確認し、なければ gh auth refresh で追加する
fn ensure_delete_repo_scope() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gh").args(["auth", "status"]).output()?;
    let status_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let has_scope = status_text
        .lines()
        .filter(|line| line.contains("Token scopes:"))
        .any(|line| line.contains("delete_repo"));

    if has_scope {
        println!("   -> delete_repo スコープ確認済みです。");
        return Ok(());
    }

    println!("   -> delete_repo スコープがありません。gh auth refresh でスコープを追加します...");
    let refresh_status = Command::new("gh")
        .args(["auth", "refresh", "-h", "github.com", "-s", "delete_repo"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if refresh_status.success() {
        println!("   -> delete_repo スコープを追加しました。処理を続行します。");
        Ok(())
    } else {
        Err("delete_repo スコープの追加が完了しませんでした。".into())
    }
}

/// gh repo edit で既存リポジトリの可視性 (public/private) を変更する
fn change_repo_visibility(
    repo_name: &str,
    visibility: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if visibility == "private" {
        println!("   注意: 非公開化するとスター数とウォッチャーは失われます。");
    }

    // gh repo edit は OWNER/REPO 形式が必須のため、OWNER 省略時は認証中ユーザーで補完する
    let full_name = if repo_name.contains('/') {
        repo_name.to_string()
    } else {
        let owner = current_gh_login()?;
        format!("{}/{}", owner, repo_name)
    };

    let output = Command::new("gh")
        .args(["repo", "edit", &full_name, "--visibility", visibility])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "リポジトリ '{}' の可視性変更に失敗しました: {}",
            full_name,
            stderr.trim()
        )
        .into());
    }
    Ok(())
}

/// 認証中の GitHub ユーザー名 (login) を取得する
fn current_gh_login() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GitHubユーザー名の取得に失敗しました: {}", stderr.trim()).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// gh repo list でリポジトリ一覧を取得し、Private/Public を判別して表示する
fn list_github_repos() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gh")
        .args([
            "repo",
            "list",
            "--limit",
            "1000",
            "--json",
            "nameWithOwner,visibility",
            "--jq",
            r#".[] | "\(.nameWithOwner)\t\(.visibility)""#,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("リポジトリ一覧の取得に失敗しました: {}", stderr.trim()).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let repos: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();

    if repos.is_empty() {
        println!("リポジトリが見つかりませんでした。");
        return Ok(());
    }

    let name_width = repos
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0);

    println!();
    println!("{:<name_width$}  VISIBILITY", "REPOSITORY");
    println!("{}  {}", "-".repeat(name_width), "-".repeat(10));
    for (name, visibility) in &repos {
        let upper = visibility.to_uppercase();
        let label = match upper.as_str() {
            "PRIVATE" => "Private",
            "PUBLIC" => "Public",
            "INTERNAL" => "Internal",
            other => other,
        };
        println!("{:<name_width$}  {}", name, label);
    }
    println!();
    println!("合計: {} 件", repos.len());

    Ok(())
}

fn run_cmd(dir: &Path, prog: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new(prog)
        .current_dir(dir)
        .args(args)
        .status()?;

    if !status.success() {
        return Err(format!("コマンド '{} {:?}' の実行に失敗しました。", prog, args).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::init_agent_project;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn creates_agent_skill_and_loop_scaffold() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "gh-repo-automator-agent-scaffold-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&dir).expect("temporary test directory should be created");

        init_agent_project(&dir, "sample-repo").expect("agent scaffold should be created");

        assert!(dir.join("README.md").is_file());
        assert!(dir.join(".github/agents/example.agent.md").is_file());
        assert!(dir.join(".github/skills/example-skill/SKILL.md").is_file());
        assert!(dir.join(".github/loops/example.md").is_file());
        let readme = fs::read_to_string(dir.join("README.md")).unwrap();
        assert!(readme.contains("# sample-repo"));

        fs::remove_dir_all(dir).expect("temporary test directory should be removed");
    }
}