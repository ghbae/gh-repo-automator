use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
enum ProjectType {
    Rust,
    Python,
}

#[derive(Debug)]
struct Config {
    repo_name: String,
    project_type: ProjectType,
    private: bool,
}

impl Config {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        let mut repo_name = None;
        let mut project_type = None;
        let mut private = true;

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
                            other => return Err(format!("不正なプロジェクト種別 '{}' です。(rust または python を指定してください)", other)),
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
                "-h" | "--help" => {
                    Self::print_help(&args[0]);
                    std::process::exit(0);
                }
                unknown => return Err(format!("不明なオプションです: {}", unknown)),
            }
            i += 1;
        }

        let repo_name = repo_name.ok_or("リポジトリ名が指定されていません。(-r <NAME> または --repo-name <NAME>)")?;
        let project_type = project_type.ok_or("プロジェクト種別が指定されていません。(-p <rust|python> または --project-type <rust|python>)")?;

        Ok(Config {
            repo_name,
            project_type,
            private,
        })
    }

    fn print_help(prog_name: &str) {
        println!("使い方: {} [オプション]", prog_name);
        println!();
        println!("オプション:");
        println!("  -r, --repo-name <NAME>      GitHubリポジトリ / プロジェクト名 (必須)");
        println!("  -p, --project-type <TYPE>   プロジェクト種別: rust または python (必須)");
        println!("      --private <BOOL>        プライベートリポジトリにするか (デフォルト: true)");
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

    // 1. プロジェクトディレクトリの作成
    let local_dir = PathBuf::from(&config.repo_name);
    if local_dir.exists() {
        return Err(format!("カレントディレクトリに '{}' ディレクトリが既に存在します。", config.repo_name).into());
    }
    fs::create_dir_all(&local_dir)?;
    println!("1. プロジェクトディレクトリを作成しました: {}", local_dir.display());

    // 2. テンプレート展開
    match config.project_type {
        ProjectType::Rust => {
            println!("2. Rustプロジェクトを初期化中 (cargo init)...");
            init_rust_project(&local_dir, &config.repo_name)?;
        }
        ProjectType::Python => {
            println!("2. Pythonプロジェクトを初期化中...");
            init_python_project(&local_dir, &config.repo_name)?;
        }
    }

    // 3. ローカルGit初期化・ブランチ名 main 化・初期コミット
    println!("3. ローカルGitの初期化と初期コミット (mainブランチ)...");
    setup_local_git(&local_dir)?;

    // 4. GitHubリポジトリ作成・プッシュ
    println!("4. GitHubリポジトリ '{}' を作成してプッシュ中...", config.repo_name);
    create_and_push_github(&local_dir, &config.repo_name, config.private)?;

    println!("\nすべて完了しました。");
    println!("- ローカル: {}", local_dir.canonicalize()?.display());
    println!("- リモート: https://github.com/<your-account>/{}", config.repo_name);

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