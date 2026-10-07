use assert_cmd::Command;
use predicates::prelude::*;

// 改写 HOME 的测试仅限 unix：dirs 在 Windows 上不读 HOME 环境变量
// （改走系统 Known Folders），隔离无效会触碰真实用户配置与网络。

/// 构造被测 CLI（bin 名取编译期包名，init 改名后自动跟随）
fn cli() -> Command {
    Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap()
}

#[test]
fn test_top_help_lists_commands() {
    cli()
        .arg("--help")
        .assert()
        .stdout(predicate::str::contains("demo"))
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("uninstall"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("config"))
        .success();
}

#[test]
fn test_no_subcommand_fails() {
    cli().assert().failure();
}

#[test]
fn test_demo_help_lists_subcommands() {
    cli()
        .arg("demo")
        .arg("--help")
        .assert()
        .stdout(predicate::str::contains("repos"))
        .stdout(predicate::str::contains("issues"))
        .stdout(predicate::str::contains("config"))
        .success();
}

#[test]
#[cfg(unix)]
fn test_demo_config_set_and_show_masks_token() {
    let home = tempfile::tempdir().unwrap();
    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", home.path());

    let result = std::panic::catch_unwind(|| {
        cli().args(["demo", "config", "init"]).assert().success();
        cli()
            .args(["demo", "config", "set", "token", "ghp-abcd1234"])
            .assert()
            .success();
        cli()
            .args(["demo", "config", "show"])
            .assert()
            .stdout(predicate::str::contains("****1234"))
            .stdout(predicate::str::contains("api.github.com"))
            .stdout(predicate::str::contains("ghp-abcd1234").not())
            .success();

        // 配置可能含凭据：写入后权限应收敛 0600（unix）
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(
            home.path()
                .join(format!(".{}", env!("CARGO_PKG_NAME")))
                .join("config.toml"),
        )
        .unwrap()
        .permissions()
        .mode();
        assert_eq!(mode & 0o777, 0o600);
    });

    std::env::set_var("HOME", old_home);
    result.unwrap();
}

#[test]
#[cfg(unix)]
fn test_demo_issues_requires_owner_repo_format() {
    // 无配置文件时 config load 报错在前，格式校验最先触发
    let home = tempfile::tempdir().unwrap();
    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", home.path());
    let result = std::panic::catch_unwind(|| {
        // 先建配置，隔离格式错误
        cli().args(["demo", "config", "init"]).assert().success();
        cli()
            .args(["demo", "issues", "badformat"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("owner/repo"));
    });
    std::env::set_var("HOME", old_home);
    result.unwrap();
}

#[test]
#[cfg(unix)]
fn test_doctor_fails_without_config() {
    let home = tempfile::tempdir().unwrap();
    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", home.path());
    let result = std::panic::catch_unwind(|| {
        cli()
            .arg("doctor")
            .assert()
            .failure()
            .stdout(predicate::str::contains("配置文件"))
            .stdout(predicate::str::contains("✗"));
    });
    std::env::set_var("HOME", old_home);
    result.unwrap();
}

#[test]
#[cfg(unix)]
fn test_log_level_option_accepted() {
    // --log-level 本身被接受：失败源于隔离 HOME 下配置文件缺失（load 报错），
    // 而非 clap usage 错误（本机存在真实 ~/.<app>/config.toml，故需 HOME 隔离）
    let home = tempfile::tempdir().unwrap();
    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", home.path());
    let result = std::panic::catch_unwind(|| {
        cli()
            .args(["--log-level", "debug", "demo", "config", "show"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("无法读取配置文件"))
            .stderr(predicate::str::contains("unexpected argument").not());
    });
    std::env::set_var("HOME", old_home);
    result.unwrap();
}

#[test]
fn test_update_check_prompts_init_when_placeholder() {
    // 模板态专用：init 填写仓库地址后本测试自动跳过（避免真实网络与误报）
    if !include_str!("../src/cli/commands/update.rs").contains("{{REPO_OWNER}}") {
        return;
    }
    // 模板态 REPO_OWNER/REPO_NAME 为占位值：应失败并提示先 init，不发起网络请求
    cli()
        .args(["update", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cargo run --bin init"));
}
