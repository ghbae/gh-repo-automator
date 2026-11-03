use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// bash 用補完スクリプト (-r の後は gh repo list からリポジトリ名を動的補完)
const BASH_COMPLETION: &str = r#"_gh_repo_automator() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    case "$prev" in
        -r|--repo-name)
            local repos
            repos=$(gh repo list --limit 1000 --json name --jq '.[].name' 2>/dev/null)
            COMPREPLY=( $(compgen -W ". $repos" -- "$cur") )
            return
            ;;
        -p|--project-type)
            COMPREPLY=( $(compgen -W "rust python agent" -- "$cur") )
            return
            ;;
        -v|--visibility)
            COMPREPLY=( $(compgen -W "public private local" -- "$cur") )
            return
            ;;
        --private)
            COMPREPLY=( $(compgen -W "true false" -- "$cur") )
            return
            ;;
        --description)
            COMPREPLY=()
            return
            ;;
        --completion)
            COMPREPLY=( $(compgen -W "bash zsh" -- "$cur") )
            return
            ;;
    esac

    COMPREPLY=( $(compgen -W "-r --repo-name -p --project-type --private -d --delete -l --list -v --visibility --description --completion -h --help" -- "$cur") )
}
complete -F _gh_repo_automator gh-repo-automator
"#;

/// zsh 用補完スクリプト
const ZSH_COMPLETION: &str = r#"_gh_repo_automator() {
    local -a repos
    case "${words[CURRENT-1]}" in
        -r|--repo-name)
            repos=(. ${(f)"$(gh repo list --limit 1000 --json name --jq '.[].name' 2>/dev/null)"})
            _describe 'repository' repos
            return
            ;;
        -p|--project-type) _values 'type' rust python agent; return ;;
        -v|--visibility) _values 'visibility' public private local; return ;;
        --private) _values 'bool' true false; return ;;
        --description) return ;;
        --completion) _values 'shell' bash zsh; return ;;
    esac
    _values 'option' \
        '-r' '--repo-name' '-p' '--project-type' '--private' \
        '-d' '--delete' '-l' '--list' '-v' '--visibility' \
        '--description' '--completion' '-h' '--help'
}
compdef _gh_repo_automator gh-repo-automator
"#;

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
    description: Option<String>,
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
        let mut description = None;

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
                            _ => {
                                return Err(
                                    "--private には true または false を指定してください。".into()
                                )
                            }
                        }
                    } else {
                        return Err("--private の値が指定されていません。".into());
                    }
                }
                "-d" | "--delete" => delete = true,
                "-l" | "--list" => list = true,
                "--description" => {
                    i += 1;
                    if i >= args.len()
                        || args[i].starts_with("--")
                        || matches!(args[i].as_str(), "-r" | "-p" | "-d" | "-l" | "-v" | "-h")
                    {
                        return Err("--description の値が指定されていません。オプションと同じ文字列は --description=<TEXT> で指定してください。".into());
                    }
                    description = Some(args[i].clone());
                }
                "--completion" => {
                    i += 1;
                    if i < args.len() {
                        match args[i].to_lowercase().as_str() {
                            "bash" => {
                                print!("{}", BASH_COMPLETION);
                                std::process::exit(0);
                            }
                            "zsh" => {
                                print!("{}", ZSH_COMPLETION);
                                std::process::exit(0);
                            }
                            other => {
                                return Err(format!(
                                    "不正なシェル '{}' です。(bash または zsh を指定してください)",
                                    other
                                ))
                            }
                        }
                    } else {
                        return Err(
                            "--completion の値が指定されていません。(bash または zsh)".into()
                        );
                    }
                }
                "-v" | "--visibility" => {
                    i += 1;
                    if i < args.len() {
                        match args[i].to_lowercase().as_str() {
                            v @ ("public" | "private" | "local") => visibility = Some(v.to_string()),
                            other => return Err(format!("不正な可視性 '{}' です。(public、private、local のいずれかを指定してください)", other)),
                        }
                    } else {
                        return Err("--visibility の値が指定されていません。".into());
                    }
                }
                "-h" | "--help" => {
                    Self::print_help(&args[0]);
                    std::process::exit(0);
                }
                value if value.starts_with("--description=") => {
                    description = Some(value["--description=".len()..].to_string());
                }
                unknown => return Err(format!("不明なオプションです: {}", unknown)),
            }
            i += 1;
        }

        if description.is_some() {
            if list || delete || visibility.as_deref() == Some("local") {
                return Err("--description は --list、--delete、または --visibility local と併用できません。".into());
            }
            if project_type.is_some() && visibility.is_some() {
                return Err("--description と --project-type を指定する新規作成では --visibility を指定できません。".into());
            }
        }

        if !list {
            if repo_name.is_none() {
                return Err(
                    "リポジトリ名が指定されていません。(-r <NAME> または --repo-name <NAME>)"
                        .into(),
                );
            }
            if !delete
                && !matches!(visibility.as_deref(), Some("public" | "private"))
                && project_type.is_none()
                && description.is_none()
            {
                return Err("プロジェクト種別が指定されていません。(-p <rust|python|agent> または --project-type <rust|python|agent>)".into());
            }
            if repo_name.as_deref() == Some(".")
                && !delete
                && visibility.is_none()
                && (description.is_none() || project_type.is_some())
            {
                return Err("-r . には --visibility local、private、public、--description または --delete を指定してください。".into());
            }
        }

        Ok(Config {
            repo_name,
            project_type,
            private,
            delete,
            list,
            visibility,
            description,
        })
    }

    fn print_help(prog_name: &str) {
        println!("使い方: {} [オプション]", prog_name);
        println!();
        println!("オプション:");
        println!("  -r, --repo-name <NAME>      リポジトリ / プロジェクト名。'.' はカレントディレクトリを対象にする");
        println!(
            "  -p, --project-type <TYPE>   プロジェクト種別: rust / python / agent (作成時・local指定時に必須)"
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
            "  -v, --visibility <VIS>      public: GitHubリポジトリを公開化、private: GitHubリモートがなければ作成してプッシュ、local: 指定名のローカルプロジェクトを作成"
        );
        println!(
            "      --description <TEXT>    GitHubリポジトリの説明を設定・変更（空文字で削除）"
        );
        println!("      --completion <SHELL>    シェル補完スクリプトを出力する: bash / zsh");
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

    if config.list {
        println!("0. GitHub CLI (gh) の認証確認中...");
        check_or_run_gh_auth()?;
        println!("GitHubリポジトリの一覧を取得します...");
        list_github_repos()?;
        return Ok(());
    }

    if config.delete {
        let repo_name = config
            .repo_name
            .as_deref()
            .expect("repo name is required when deleting");
        let current_name = if repo_name == "." {
            Some(current_github_repository(&env::current_dir()?)?)
        } else {
            None
        };
        let repo_name = current_name.as_deref().unwrap_or(repo_name);
        println!("0. GitHub CLI (gh) の認証確認中...");
        check_or_run_gh_auth()?;
        println!("リポジトリ削除権限 (delete_repo スコープ) を確認中...");
        ensure_delete_repo_scope()?;
        println!("GitHubリポジトリ '{}' の削除を開始します...", repo_name);
        let full_name = delete_github_repo(repo_name)?;
        println!("削除結果を確認中...");
        verify_repo_deleted(&full_name)?;
        println!(
            "GitHubリポジトリ '{}' を削除しました。(削除済みであることを確認)",
            full_name
        );
        return Ok(());
    }

    if config.description.is_some() && config.visibility.is_none() && config.project_type.is_none()
    {
        let requested_name = config
            .repo_name
            .as_deref()
            .expect("repo name is required when changing description");
        let current_name = if requested_name == "." {
            Some(current_github_repository(&env::current_dir()?)?)
        } else {
            None
        };
        let repo_name = current_name.as_deref().unwrap_or(requested_name);
        println!("0. GitHub CLI (gh) の認証確認中...");
        check_or_run_gh_auth()?;
        change_repo_settings(repo_name, None, config.description.as_deref())?;
        println!("GitHubリポジトリ '{}' の説明を変更しました。", repo_name);
        return Ok(());
    }

    if let Some(visibility) = &config.visibility {
        if visibility == "local" {
            // local creation continues through the shared project creation flow below.
        } else {
            let repo_name = config
                .repo_name
                .as_deref()
                .expect("repo name is required when changing visibility");
            let current_dir = env::current_dir()?;
            if repo_name == "." {
                if visibility == "private" {
                    let root = git_repository_root(&current_dir)?;
                    match github_origin_url(&root)? {
                        Some(url) => {
                            let name = parse_github_repository(&url)?;
                            println!("0. GitHub CLI (gh) の認証確認中...");
                            check_or_run_gh_auth()?;
                            println!("GitHubリポジトリ '{}' を非公開に変更します...", name);
                            change_repo_settings(
                                &name,
                                Some(visibility),
                                config.description.as_deref(),
                            )?;
                            println!("GitHubリポジトリ '{}' を非公開に変更しました。", name);
                        }
                        None => {
                            let name = directory_name(&root)?;
                            println!("0. GitHub CLI (gh) の認証確認中...");
                            check_or_run_gh_auth()?;
                            println!(
                                "GitHubリポジトリ '{}' を非公開で作成してプッシュします...",
                                name
                            );
                            create_and_push_github(
                                &root,
                                &name,
                                true,
                                config.description.as_deref(),
                            )?;
                            println!(
                                "GitHubリポジトリ '{}' を非公開で作成し、プッシュしました。",
                                name
                            );
                        }
                    }
                } else {
                    let name = current_github_repository(&current_dir)?;
                    println!("0. GitHub CLI (gh) の認証確認中...");
                    check_or_run_gh_auth()?;
                    println!("GitHubリポジトリ '{}' を公開に変更します...", name);
                    change_repo_settings(&name, Some(visibility), config.description.as_deref())?;
                    println!("GitHubリポジトリ '{}' を公開に変更しました。", name);
                }
                return Ok(());
            }
            println!("0. GitHub CLI (gh) の認証確認中...");
            check_or_run_gh_auth()?;
            if visibility == "private" {
                match github_origin_url(&current_dir)? {
                    Some(remote_url) => {
                        println!(
                        "GitHubリモート '{}' を検出しました。リポジトリ '{}' を非公開に変更します...",
                        remote_url, repo_name
                    );
                        change_repo_settings(
                            repo_name,
                            Some(visibility),
                            config.description.as_deref(),
                        )?;
                        println!(
                            "GitHubリポジトリ '{}' を非公開 (Private) に変更しました。",
                            repo_name
                        );
                    }
                    None => {
                        println!(
                        "GitHubリモートがありません。GitHubリポジトリ '{}' を非公開で作成してプッシュします...",
                        repo_name
                    );
                        create_and_push_github(
                            &current_dir,
                            repo_name,
                            true,
                            config.description.as_deref(),
                        )?;
                        println!(
                            "GitHubリポジトリ '{}' を非公開 (Private) で作成し、プッシュしました。",
                            repo_name
                        );
                    }
                }
                return Ok(());
            }

            println!(
                "GitHubリポジトリ '{}' の可視性を '{}' に変更します...",
                repo_name, visibility
            );
            change_repo_settings(repo_name, Some(visibility), config.description.as_deref())?;
            println!(
                "GitHubリポジトリ '{}' を {} に変更しました。",
                repo_name,
                if visibility == "private" {
                    "非公開 (Private)"
                } else {
                    "公開 (Public)"
                }
            );
            return Ok(());
        }
    }

    let is_local_creation = config.visibility.as_deref() == Some("local");
    if !is_local_creation {
        // 0. GitHub CLI (gh) の認証状態を確認（未認証ならその場でログイン実行）
        println!("0. GitHub CLI (gh) の認証確認中...");
        check_or_run_gh_auth()?;
    }

    let requested_name = config
        .repo_name
        .as_deref()
        .expect("repo name is required when creating");

    // 1. プロジェクトディレクトリの作成
    let (local_dir, resolved_name) = if requested_name == "." {
        let dir = env::current_dir()?;
        let name = directory_name(&dir)?;
        check_current_directory_for_init(
            &dir,
            &name,
            config
                .project_type
                .as_ref()
                .expect("project type is required"),
        )?;
        println!("1. カレントディレクトリを使用します: {}", dir.display());
        (dir, name)
    } else {
        let dir = PathBuf::from(requested_name);
        if dir.exists() {
            return Err(format!(
                "カレントディレクトリに '{}' ディレクトリが既に存在します。",
                requested_name
            )
            .into());
        }
        fs::create_dir_all(&dir)?;
        println!(
            "1. プロジェクトディレクトリを作成しました: {}",
            dir.display()
        );
        (dir, requested_name.to_string())
    };
    let repo_name = resolved_name.as_str();

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

    if is_local_creation {
        println!("\nローカルGitリポジトリの作成が完了しました。");
        println!("- ローカル: {}", local_dir.canonicalize()?.display());
        return Ok(());
    }

    // 4. GitHubリポジトリ作成・プッシュ
    println!(
        "4. GitHubリポジトリ '{}' を作成してプッシュ中...",
        repo_name
    );
    create_and_push_github(
        &local_dir,
        repo_name,
        config.private,
        config.description.as_deref(),
    )?;

    println!("\nすべて完了しました。");
    println!("- ローカル: {}", local_dir.canonicalize()?.display());
    println!(
        "- リモート: https://github.com/<your-account>/{}",
        repo_name
    );

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
            println!(
                "   -> GitHub CLI 未ログインです。ログイン処理 (gh auth login) を起動します..."
            );
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
        .args(["init", "--vcs", "none", "--name", repo_name])
        .status()?;

    if !status.success() {
        return Err("cargo init に失敗しました。".into());
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(".gitignore"))?
        .write_all(b"/target\nCargo.lock\n")?;

    let readme = format!(
        "# {}\n\nA Rust project generated by gh-repo-automator.\n",
        repo_name
    );
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

    let gitignore =
        "__pycache__/\n*.py[cod]\n*$py.class\n.venv/\nvenv/\ndist/\nbuild/\n*.egg-info/\n";
    File::create(dir.join(".gitignore"))?.write_all(gitignore.as_bytes())?;

    let readme = format!(
        "# {}\n\nA Python project generated by gh-repo-automator.\n",
        repo_name
    );
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
    run_cmd(
        dir,
        "git",
        &["commit", "--allow-empty", "-m", "feat: initial commit"],
    )?;
    Ok(())
}

fn directory_name(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or("ディレクトリ名をリポジトリ名として取得できません。")?;
    Ok(name.to_string())
}

fn git_repository_root(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Gitリポジトリのルートを取得できません: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    let root = String::from_utf8(output.stdout)?;
    Ok(PathBuf::from(root.trim_end_matches(['\n', '\r'])))
}

fn parse_github_repository(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("git://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .ok_or("origin のURLから github.com のリポジトリを特定できません。")?;
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path
        .split_once('/')
        .ok_or("origin のURLに OWNER/REPO が含まれていません。")?;
    if owner.is_empty()
        || repo.is_empty()
        || matches!(repo, "." | "..")
        || !owner
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || !repo
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
    {
        return Err("origin のURLに有効な OWNER/REPO が含まれていません。".into());
    }
    Ok(format!("{owner}/{repo}"))
}

fn current_github_repository(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let root = git_repository_root(dir)?;
    let url = github_origin_url(&root)?
        .ok_or("origin が設定されていません。GitHubリポジトリを特定できません。")?;
    parse_github_repository(&url)
}

fn check_current_directory_for_init(
    dir: &Path,
    name: &str,
    project_type: &ProjectType,
) -> Result<(), Box<dyn std::error::Error>> {
    match fs::symlink_metadata(dir.join(".git")) {
        Ok(_) => return Err("カレントディレクトリには既に .git が存在します。".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let output = Command::new("git")
        .current_dir(dir)
        .env("LC_ALL", "C")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if output.status.success() {
        return Err("カレントディレクトリは既にGitリポジトリ内にあります。".into());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("not a git repository") {
        return Err(format!("Gitリポジトリの確認に失敗しました: {}", stderr.trim()).into());
    }

    let mut paths = vec![PathBuf::from("README.md")];
    match project_type {
        ProjectType::Rust => paths.extend([
            PathBuf::from("Cargo.toml"),
            PathBuf::from("src"),
            PathBuf::from(".gitignore"),
        ]),
        ProjectType::Python => paths.extend([
            PathBuf::from("pyproject.toml"),
            PathBuf::from(".gitignore"),
            PathBuf::from(name.replace('-', "_")),
        ]),
        ProjectType::Agent => paths.push(PathBuf::from(".github")),
    }
    for path in paths {
        match fs::symlink_metadata(dir.join(&path)) {
            Ok(_) => return Err(format!("既存の '{}' は上書きできません。", path.display()).into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn github_origin_url(dir: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let git_status = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !git_status.status.success() {
        return Err(format!(
            "'{}' はGitリポジトリではありません。先に -r . -p <TYPE> -v local で初期化してください。",
            dir.display()
        )
        .into());
    }

    let remotes = Command::new("git")
        .current_dir(dir)
        .arg("remote")
        .output()?;
    if !remotes.status.success() {
        let stderr = String::from_utf8_lossy(&remotes.stderr);
        return Err(format!("Gitリモートの取得に失敗しました: {}", stderr.trim()).into());
    }
    if !String::from_utf8_lossy(&remotes.stdout)
        .lines()
        .any(|remote| remote == "origin")
    {
        return Ok(None);
    }

    let origin = Command::new("git")
        .current_dir(dir)
        .args(["remote", "get-url", "origin"])
        .output()?;
    if !origin.status.success() {
        let stderr = String::from_utf8_lossy(&origin.stderr);
        return Err(format!("origin リモートの取得に失敗しました: {}", stderr.trim()).into());
    }

    let origin_url = String::from_utf8_lossy(&origin.stdout).trim().to_string();
    if is_github_remote_url(&origin_url) {
        Ok(Some(origin_url))
    } else {
        Err(format!(
            "origin リモート '{}' はGitHubではありません。既存のリモートを変更しないため、プッシュを中止しました。",
            origin_url
        )
        .into())
    }
}

fn is_github_remote_url(url: &str) -> bool {
    url.contains("github.com/") || url.contains("github.com:")
}

fn create_and_push_github(
    dir: &Path,
    repo_name: &str,
    is_private: bool,
    description: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let visibility_flag = if is_private { "--private" } else { "--public" };

    let mut args = vec![
        "repo",
        "create",
        repo_name,
        visibility_flag,
        "--source=.",
        "--remote=origin",
        "--push",
    ];
    if let Some(description) = description {
        args.extend(["--description", description]);
    }
    run_cmd(dir, "gh", &args)?;

    Ok(())
}

/// gh repo delete でリポジトリを削除し、解決済みの OWNER/REPO 名を返す
fn delete_github_repo(repo_name: &str) -> Result<String, Box<dyn std::error::Error>> {
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
        return Err(format!(
            "GitHubリポジトリ '{}' の削除に失敗しました。(確認プロンプトには '{}' と完全一致で入力してください)",
            full_name, full_name
        )
        .into());
    }
    Ok(full_name)
}

/// gh repo view でリポジトリが削除済みであることを検証する
fn verify_repo_deleted(full_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gh")
        .args(["repo", "view", full_name, "--json", "name"])
        .output()?;

    if output.status.success() {
        return Err(format!(
            "リポジトリ '{}' がまだ存在します。削除は完了していません。",
            full_name
        )
        .into());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Could not resolve") || stderr.contains("HTTP 404") {
        // リポジトリが見つからない = 削除成功
        Ok(())
    } else {
        Err(format!(
            "リポジトリ '{}' の削除確認に失敗しました: {}",
            full_name,
            stderr.trim()
        )
        .into())
    }
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

/// gh repo edit で既存リポジトリの可視性と説明を変更する
fn change_repo_settings(
    repo_name: &str,
    visibility: Option<&str>,
    description: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if visibility == Some("private") {
        println!("   注意: 非公開化するとスター数とウォッチャーは失われます。");
    }

    // gh repo edit は OWNER/REPO 形式が必須のため、OWNER 省略時は認証中ユーザーで補完する
    let full_name = if repo_name.contains('/') {
        repo_name.to_string()
    } else {
        let owner = current_gh_login()?;
        format!("{}/{}", owner, repo_name)
    };

    let mut args = vec!["repo", "edit", &full_name];
    if let Some(visibility) = visibility {
        args.extend(["--visibility", visibility]);
    }
    if let Some(description) = description {
        args.extend(["--description", description]);
    }
    let output = Command::new("gh").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "リポジトリ '{}' の設定変更に失敗しました: {}",
            full_name,
            stderr.trim()
        )
        .into());
    }
    if visibility.is_some() && description.is_some() {
        println!("GitHubリポジトリ '{}' の説明も変更しました。", full_name);
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
    let status = Command::new(prog).current_dir(dir).args(args).status()?;

    if !status.success() {
        return Err(format!("コマンド '{} {:?}' の実行に失敗しました。", prog, args).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{init_agent_project, is_github_remote_url, parse_github_repository};
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

    #[test]
    fn identifies_github_remote_urls() {
        assert!(is_github_remote_url(
            "https://github.com/example/example-repository.git"
        ));
        assert!(is_github_remote_url(
            "git@github.com:example/example-repository.git"
        ));
        assert!(!is_github_remote_url(
            "https://gitlab.com/example/example-repository.git"
        ));
    }

    #[test]
    fn resolves_github_origin_without_trusting_path_or_userinfo() {
        for url in [
            "https://github.com/owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
            "git@github.com:owner/repo.git",
        ] {
            assert_eq!(parse_github_repository(url).unwrap(), "owner/repo");
        }
        for url in [
            "https://example.com/github.com/owner/repo",
            "https://github.com@evil.example/owner/repo",
            "git@github.com:owner/repo/extra",
            "git@github.com:owner/..",
        ] {
            assert!(parse_github_repository(url).is_err());
        }
    }
}
