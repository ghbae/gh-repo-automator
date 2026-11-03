#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gh-repo-automator-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn git(dir: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test User")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn cli(dir: &Path, args: &[&str], bin_dir: Option<&Path>, log: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gh-repo-automator"));
    command
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test User")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args);
    if let Some(bin_dir) = bin_dir {
        let mut paths = vec![bin_dir.to_path_buf()];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        command.env("PATH", std::env::join_paths(paths).unwrap());
    }
    if let Some(log) = log {
        command.env("GH_TEST_LOG", log);
    }
    command.output().unwrap()
}

fn fake_gh(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GH_TEST_LOG\"\n\
         if [ \"$1 $2\" = 'repo edit' ] || [ \"$1 $2\" = 'repo create' ]; then printf 'argument: <%s>\\n' \"$@\" >> \"$GH_TEST_LOG\"; fi\n\
         if [ \"$1 $2\" = 'repo create' ]; then printf 'create cwd: %s\\n' \"$PWD\" >> \"$GH_TEST_LOG\"; fi\n\
         if [ \"$1 $2\" = 'auth status' ]; then echo 'Token scopes: delete_repo'; fi\n\
         if [ \"$1 $2\" = 'api user' ]; then echo 'test-owner'; fi\n\
         if [ \"$1 $2\" = 'repo edit' ] && [ -f \"$GH_TEST_LOG.fail-edit\" ]; then echo 'edit failed' >&2; exit 1; fi\n\
         if [ \"$1 $2\" = 'repo view' ]; then echo 'HTTP 404' >&2; exit 1; fi\n\
         exit 0\n",
    )
    .unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_dot_creates_templates_and_initial_commit_without_overwriting() {
    for (kind, expected) in [
        ("python", "pyproject.toml"),
        ("rust", "Cargo.toml"),
        ("agent", ".github/agents/example.agent.md"),
    ] {
        let temp = TestDir::new();
        let project = temp.path().join("sample-project");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("existing.txt"), "preserve").unwrap();
        let output = cli(
            &project,
            &["-r", ".", "-p", kind, "-v", "local"],
            None,
            None,
        );
        assert_success(&output);
        assert!(project.join(expected).is_file());
        assert_eq!(
            fs::read_to_string(project.join("existing.txt")).unwrap(),
            "preserve"
        );
        assert_eq!(
            git(&project, &["branch", "--show-current"]).stdout,
            b"main\n"
        );
        assert_eq!(
            git(&project, &["log", "-1", "--format=%s"]).stdout,
            b"feat: initial commit\n"
        );
        git(&project, &["cat-file", "-e", "HEAD:existing.txt"]);
        let repeated = cli(
            &project,
            &["-r", ".", "-p", kind, "-v", "local"],
            None,
            None,
        );
        assert!(!repeated.status.success());
    }
}

#[test]
fn local_dot_refuses_collisions_and_requires_project_type() {
    let temp = TestDir::new();
    let project = temp.path().join("sample-project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("README.md"), "do not overwrite").unwrap();
    let output = cli(
        &project,
        &["-r", ".", "-p", "python", "-v", "local"],
        None,
        None,
    );
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(project.join("README.md")).unwrap(),
        "do not overwrite"
    );
    assert!(!project.join(".git").exists());
    let output = cli(&project, &["-r", ".", "-v", "local"], None, None);
    assert!(!output.status.success());
}

#[test]
fn dot_upload_uses_root_name_and_source_from_subdirectory() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    let project = temp.path().join("my-project");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    git(&project, &["commit", "--allow-empty", "-m", "initial"]);
    let subdir = project.join("subdir");
    fs::create_dir(&subdir).unwrap();
    let output = cli(
        &subdir,
        &["-r", ".", "-v", "private"],
        Some(&bin),
        Some(&log),
    );
    assert_success(&output);
    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.contains("repo create my-project --private --source=. --remote=origin --push"));
    assert!(calls.contains(&format!("create cwd: {}", project.display())));
    assert!(!calls.contains("repo edit"));
}

#[test]
fn named_local_creation_still_creates_a_child_directory() {
    let temp = TestDir::new();
    let output = cli(
        temp.path(),
        &["-r", "named-project", "-p", "rust", "-v", "local"],
        None,
        None,
    );
    assert_success(&output);
    let project = temp.path().join("named-project");
    assert!(project.join("Cargo.toml").is_file());
    assert!(project.join(".gitignore").is_file());
    assert_eq!(
        git(&project, &["log", "-1", "--format=%s"]).stdout,
        b"feat: initial commit\n"
    );
}

#[test]
fn local_dot_refuses_nested_git_repository() {
    let temp = TestDir::new();
    git(temp.path(), &["init"]);
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let output = cli(
        &subdir,
        &["-r", ".", "-p", "python", "-v", "local"],
        None,
        None,
    );
    assert!(!output.status.success());
    assert!(!subdir.join(".git").exists());
    assert!(!subdir.join("pyproject.toml").exists());
}

#[test]
fn dot_uses_github_origin_for_visibility_and_delete() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    let project = temp.path().join("different-local-name");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    git(
        &project,
        &[
            "remote",
            "add",
            "origin",
            "git@github.com:real-owner/real-repo.git",
        ],
    );
    let subdir = project.join("subdir");
    fs::create_dir(&subdir).unwrap();
    for args in [
        &["-r", ".", "-v", "private"][..],
        &["-r", ".", "-v", "public"][..],
        &["-r", ".", "--delete"][..],
    ] {
        assert_success(&cli(&subdir, args, Some(&bin), Some(&log)));
    }
    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.contains("repo edit real-owner/real-repo --visibility private"));
    assert!(calls.contains("repo edit real-owner/real-repo --visibility public"));
    assert!(calls.contains("repo delete real-owner/real-repo"));
    assert!(calls.contains("repo view real-owner/real-repo --json name"));
    assert!(!calls.contains("repo create"));
}

#[test]
fn dot_rejects_non_github_or_missing_origin_for_existing_operations() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    for args in [
        &["-r", ".", "-v", "public"][..],
        &["-r", ".", "--delete"][..],
    ] {
        assert!(!cli(&project, args, Some(&bin), Some(&log)).status.success());
    }
    git(
        &project,
        &[
            "remote",
            "add",
            "origin",
            "https://evil.example/github.com/owner/repo",
        ],
    );
    assert!(!cli(
        &project,
        &["-r", ".", "-v", "private"],
        Some(&bin),
        Some(&log)
    )
    .status
    .success());
    assert!(!log.exists());
}

#[test]
fn description_edits_named_and_current_repositories_and_clears_text() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    git(
        &project,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/origin-owner/origin-repo.git",
        ],
    );
    for args in [
        &["-r", "OWNER/REPO", "--description", "説明 with spaces"][..],
        &["-r", "repo", "--description", "named"][..],
        &["-r", ".", "--description", ""][..],
        &["-r", ".", "--description=--delete"][..],
        &[
            "-r",
            ".",
            "-v",
            "public",
            "--description",
            "public description",
        ][..],
        &[
            "-r",
            ".",
            "-v",
            "private",
            "--description",
            "private description",
        ][..],
    ] {
        assert_success(&cli(&project, args, Some(&bin), Some(&log)));
    }
    let calls = fs::read_to_string(&log).unwrap();
    assert!(calls.contains("repo edit OWNER/REPO --description 説明 with spaces"));
    assert!(calls.contains("argument: <説明 with spaces>"));
    assert!(calls.contains("repo edit test-owner/repo --description named"));
    assert!(calls.contains("repo edit origin-owner/origin-repo --description "));
    assert!(calls.contains("argument: <>"));
    assert!(calls.contains("argument: <--delete>"));
    assert!(calls.contains(
        "repo edit origin-owner/origin-repo --visibility public --description public description"
    ));
    assert!(calls.contains(
        "repo edit origin-owner/origin-repo --visibility private --description private description"
    ));
    assert!(!calls.contains("repo create"));

    fs::write(temp.path().join("gh.log.fail-edit"), "").unwrap();
    let failed = cli(
        &project,
        &["-r", ".", "--description", "failed"],
        Some(&bin),
        Some(&log),
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("edit failed"));
}

#[test]
fn description_is_passed_during_new_repository_creation_and_upload() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    assert_success(&cli(
        temp.path(),
        &[
            "-r",
            "new-app",
            "-p",
            "python",
            "--private",
            "false",
            "--description",
            "new description",
        ],
        Some(&bin),
        Some(&log),
    ));
    let project = temp.path().join("local-app");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    git(&project, &["commit", "--allow-empty", "-m", "initial"]);
    assert_success(&cli(
        &project,
        &[
            "-r",
            ".",
            "-v",
            "private",
            "--description",
            "upload description",
        ],
        Some(&bin),
        Some(&log),
    ));
    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.contains("repo create new-app --public --source=. --remote=origin --push --description new description"));
    assert!(calls.contains("repo create local-app --private --source=. --remote=origin --push --description upload description"));
    assert!(calls.contains("argument: <new description>"));
    assert!(calls.contains("argument: <upload description>"));
}

#[test]
fn description_rejects_invalid_combinations_without_github_calls() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    for args in [
        &["-r", "repo", "--description"][..],
        &["-r", "repo", "--description", "--delete"][..],
        &["-r", "repo", "--description", "--unknown"][..],
        &["-r", "repo", "--description", "text", "--delete"][..],
        &["--list", "--description", "text"][..],
        &[
            "-r",
            "repo",
            "-p",
            "python",
            "-v",
            "local",
            "--description",
            "text",
        ][..],
        &[
            "-r",
            "repo",
            "-p",
            "python",
            "-v",
            "public",
            "--description",
            "text",
        ][..],
        &["--description", "text"][..],
    ] {
        let output = cli(temp.path(), args, Some(&bin), Some(&log));
        assert!(!output.status.success(), "unexpected success for {args:?}");
    }
    assert!(!log.exists());
}

#[test]
fn description_requires_a_github_origin_for_dot() {
    let temp = TestDir::new();
    let bin = fake_gh(temp.path());
    let log = temp.path().join("gh.log");
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    git(&project, &["init"]);
    let output = cli(
        &project,
        &["-r", ".", "--description", "text"],
        Some(&bin),
        Some(&log),
    );
    assert!(!output.status.success());
    assert!(!log.exists());
}
