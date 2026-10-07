//! 交互式初始化：替换模板占位符（幂等，可重复运行）。
//!
//! 用法：cargo run --bin init
//! 占位符：{{APP_NAME}} {{APP_DESCRIPTION}} {{AUTHOR}} {{REPO_OWNER}} {{REPO_NAME}}
//! 扫描 .toml/.md/.yml + 定点 src/cli/commands/update.rs 与 LICENSE；
//! Cargo.toml 的 name/default-run 用内置默认值 my-cli 定点替换（见 CLAUDE.md"脚手架专有"）。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 本次 init 输入的记录文件（幂等重跑的旧值来源，已 gitignore）
const INIT_RECORD: &str = "starter-init.json";
/// 模板出厂默认 crate 名（Cargo.toml name 的"内置旧值"）
const DEFAULT_CRATE_NAME: &str = "my-cli";
/// 按扩展名参与占位符扫描的文件
const SCAN_EXTS: [&str; 3] = ["toml", "md", "yml"];
/// 不参与扫描的目录
const IGNORE_DIRS: [&str; 4] = ["target", ".git", ".github", "docs"];
/// 按文件名不参与扫描的文件（从不含占位符，纳入仅有旧值子串替换的腐蚀风险，
/// 如旧 REPO_NAME "tool" 会把 [toolchain] 连带成 [renamedchain] 使 rustup 报错）
const SKIP_FILES: [&str; 1] = ["rust-toolchain.toml"];
/// 定点替换文件（无扩展名或含源码占位符例外）
const PINNED_FILES: [&str; 2] = ["src/cli/commands/update.rs", "LICENSE"];
/// 占位符键集合（{{KEY}} 形态）
const PLACEHOLDERS: [&str; 5] = [
    "APP_NAME",
    "APP_DESCRIPTION",
    "AUTHOR",
    "REPO_OWNER",
    "REPO_NAME",
];

/// init 输入记录（starter-init.json：键 → 上次输入值）
#[derive(Debug, Default, Serialize, Deserialize)]
struct InitRecord(BTreeMap<String, String>);

fn main() -> Result<()> {
    let root = find_project_root()?;
    println!("=== 模板初始化（cargo run --bin init）===\n");

    let old = load_record(&root)?;
    let values = ask_values(&old)?;
    let changed = apply(&root, &values, &old)?;
    save_record(&root, &values)?;

    println!(
        "\n已更新 {} 个文件（输入记录在 {INIT_RECORD}，重复运行可覆盖更新）",
        changed.len()
    );
    if values
        .get("REPO_OWNER")
        .map(String::as_str)
        .unwrap_or("")
        .is_empty()
    {
        println!("ℹ 未配置仓库地址：`update` 自更新不可用（运行时会提示先 init），可稍后重跑本命令补填。");
    }
    println!(
        "\n✓ 初始化完成！下一步：\n  cargo build\n  {} --help",
        values
            .get("APP_NAME")
            .cloned()
            .unwrap_or_else(|| DEFAULT_CRATE_NAME.to_string())
    );
    Ok(())
}

/// 从当前目录向上找含 Cargo.toml 的项目根
fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("无法获取当前目录")?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!("未找到项目根（向上未发现 Cargo.toml）");
        }
    }
}

/// 交互问答：应用名（校验）→ 描述 → 作者 → 仓库地址（解析 owner/name）
fn ask_values(old: &InitRecord) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    let get = |k: &str| old.0.get(k).cloned().unwrap_or_default();

    // 1. 应用名（crate 名，非法重问）
    let mut app_name = ask_line(
        "应用名（crate 名）",
        or_default(&get("APP_NAME"), DEFAULT_CRATE_NAME.to_string()),
    )?;
    while !usable_crate_name(&app_name) {
        if app_name == "init" {
            eprintln!(
                "  ✗ 应用名不能用 init：与 src/bin/init.rs 重名会导致 duplicate binary，构建失败"
            );
        } else {
            eprintln!("  ✗ 仅允许小写字母开头的小写字母/数字/-/_ 组合");
        }
        app_name = ask_line("应用名", app_name)?;
    }
    values.insert("APP_NAME".to_string(), app_name);

    // 2. 描述
    let desc = ask_line(
        "描述",
        or_default(&get("APP_DESCRIPTION"), "A CLI tool".to_string()),
    )?;
    values.insert("APP_DESCRIPTION".to_string(), desc);

    // 3. 作者（首次默认取 git 配置）
    let author = ask_line("作者", or_default(&get("AUTHOR"), git_author()))?;
    values.insert("AUTHOR".to_string(), author);

    // 4. 仓库地址（可留空 = 不启用自更新）
    let url_fallback = {
        let (o, n) = (get("REPO_OWNER"), get("REPO_NAME"));
        if o.is_empty() || n.is_empty() {
            String::new()
        } else {
            format!("https://github.com/{o}/{n}")
        }
    };
    let repo_url = ask_line(
        "仓库地址 https://github.com/<owner>/<name>（可留空）",
        url_fallback,
    )?;
    let (owner, name) = match parse_repo_ref(&repo_url) {
        Some(pair) => pair,
        None => {
            if !repo_url.trim().is_empty() {
                eprintln!("  ⚠ 无法解析为 owner/name，按未配置处理");
            }
            (String::new(), String::new())
        }
    };
    values.insert("REPO_URL".to_string(), repo_url.trim().to_string());
    values.insert("REPO_OWNER".to_string(), owner);
    values.insert("REPO_NAME".to_string(), name);
    Ok(values)
}

/// 读一行输入：空输入返回 fallback
fn ask_line(prompt: &str, fallback: String) -> Result<String> {
    let suffix = if fallback.is_empty() {
        String::new()
    } else {
        format!(" ({fallback})")
    };
    print!("{prompt}{suffix}: ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    let n = std::io::stdin()
        .read_line(&mut line)
        .context("无法读取输入")?;
    // EOF（0 字节且输入为空）：管道输入已耗尽，报错退出；
    // 否则上层"非法重问"循环会反复拿到 fallback 原值，陷入热循环
    if n == 0 && line.is_empty() {
        anyhow::bail!("输入已耗尽（EOF）：请补全全部输入后重试，或在终端交互运行");
    }
    let v = line.trim().to_string();
    Ok(if v.is_empty() { fallback } else { v })
}

/// 当前值为空时用默认值
fn or_default(current: &str, default: String) -> String {
    if current.is_empty() {
        default
    } else {
        current.to_string()
    }
}

/// crate 名合法性：小写字母开头，仅小写字母/数字/`-`/`_`
fn valid_crate_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && name
            .chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-' | '_'))
}

/// 应用名可用性：合法 crate 名，且不与 init 二进制重名
/// （包内已有 src/bin/init.rs，主 bin 再叫 init 会 duplicate binary，构建失败）
fn usable_crate_name(name: &str) -> bool {
    valid_crate_name(name) && name != "init"
}

/// TOML 基本字符串转义：值含 `"` 或 `\` 时避免产出非法 TOML（仅用于 Cargo.toml 替换）
fn escape_toml_string(v: &str) -> String {
    v.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 解析仓库引用为 (owner, name)：支持完整 URL 与 owner/name 两种写法；
/// host 非 github.com 时打印警告（自更新仅支持 GitHub Releases）但仍解析
fn parse_repo_ref(input: &str) -> Option<(String, String)> {
    let input = input.trim().trim_end_matches(".git");
    let (path, had_scheme) = match input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
    {
        Some(p) => (p, true),
        None => (input, false),
    };
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        [owner, name] if !had_scheme => {
            // SSH 写法（git@github.com:owner/name）的段含 @/:，按无法解析处理，
            // 避免静默产出废 owner
            if owner.contains('@')
                || owner.contains(':')
                || name.contains('@')
                || name.contains(':')
            {
                return None;
            }
            Some((owner.to_string(), name.to_string()))
        }
        [host, owner, name] => {
            if !host.contains('.') {
                return None;
            }
            if *host != "github.com" {
                eprintln!("  ⚠ 非 github.com 地址（{host}）：自更新仅支持 GitHub Releases，其余命令不受影响");
            }
            Some((owner.to_string(), name.to_string()))
        }
        _ => None,
    }
}

/// 从 git 配置取作者（name <email>），取不到返回 unknown
fn git_author() -> String {
    let query = |key: &str| {
        std::process::Command::new("git")
            .args(["config", key])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    match (query("user.name"), query("user.email")) {
        (Some(n), Some(e)) => format!("{n} <{e}>"),
        (Some(n), None) => n,
        (None, _) => "unknown".to_string(),
    }
}

/// 执行替换：占位符 + 旧值 + Cargo.toml name/default-run 定点，返回被修改的文件
fn apply(root: &Path, values: &BTreeMap<String, String>, old: &InitRecord) -> Result<Vec<PathBuf>> {
    let files = collect_files(root)?;
    let mut changed = Vec::new();

    for file in files {
        let content = fs::read_to_string(&file).with_context(|| file.display().to_string())?;
        let mut next = content.clone();
        let is_cargo_toml = file.file_name().is_some_and(|n| n == "Cargo.toml");

        // 1) 占位符与上次旧值替换（幂等：改主意后重跑会把旧值一并换掉）
        for key in PLACEHOLDERS {
            let new_val = values.get(key).map(String::as_str).unwrap_or("");
            // Cargo.toml 是 TOML 源：字符串值含 `"` 或 `\` 必须转义，否则产出非法 TOML；
            // 其余文件（README/CLAUDE.md 等）保留原字符。APP_NAME 已过 crate 名校验、
            // REPO_OWNER/REPO_NAME 为 URL 路径段，均不会含引号，无需转义。
            let needs_escape = is_cargo_toml && matches!(key, "APP_DESCRIPTION" | "AUTHOR");
            let new_repl = if needs_escape {
                escape_toml_string(new_val)
            } else {
                new_val.to_string()
            };
            next = next.replace(&format!("{{{{{key}}}}}"), &new_repl);
            if let Some(old_val) = old.0.get(key) {
                if !old_val.is_empty() && old_val != new_val {
                    // 旧值替换同样用转义形态匹配（上次写入文件的就是转义后的串）
                    let old_repl = if needs_escape {
                        escape_toml_string(old_val)
                    } else {
                        old_val.clone()
                    };
                    next = next.replace(&old_repl, &new_repl);
                }
            }
        }

        // 2) Cargo.toml：crate name 不用占位符，定点替换 name/default-run = "<旧名>"
        //    （旧名 = 上次输入，首次 init = 内置默认 my-cli）
        if file.file_name().is_some_and(|n| n == "Cargo.toml") {
            let old_name = old
                .0
                .get("APP_NAME")
                .cloned()
                .unwrap_or_else(|| DEFAULT_CRATE_NAME.to_string());
            let new_name = values
                .get("APP_NAME")
                .cloned()
                .unwrap_or_else(|| DEFAULT_CRATE_NAME.to_string());
            if valid_crate_name(&old_name) && old_name != new_name {
                next = next.replace(
                    &format!("name = \"{old_name}\""),
                    &format!("name = \"{new_name}\""),
                );
                // default-run（若有）与 name 同步改名；无该行时 replace 为 no-op
                next = next.replace(
                    &format!("default-run = \"{old_name}\""),
                    &format!("default-run = \"{new_name}\""),
                );
            }
        }

        if next != content {
            fs::write(&file, &next).with_context(|| file.display().to_string())?;
            changed.push(file);
        }
    }
    Ok(changed)
}

/// 收集待替换文件：按扩展名扫描（忽略构建/文档目录）+ 定点文件
fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(root, &mut out)?;
    for rel in PINNED_FILES {
        let p = root.join(rel);
        if p.exists() {
            out.push(p);
        }
    }
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !IGNORE_DIRS.contains(&name.as_ref()) {
                walk(&path, out)?;
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if SCAN_EXTS.contains(&ext) && !SKIP_FILES.contains(&name.as_ref()) {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// 读取上次记录（不存在或损坏按首次处理）
fn load_record(root: &Path) -> Result<InitRecord> {
    let path = root.join(INIT_RECORD);
    if !path.exists() {
        return Ok(InitRecord::default());
    }
    let content = fs::read_to_string(&path).with_context(|| format!("无法读取 {INIT_RECORD}"))?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

/// 写入本次记录
fn save_record(root: &Path, values: &BTreeMap<String, String>) -> Result<()> {
    fs::write(
        root.join(INIT_RECORD),
        serde_json::to_string_pretty(values)? + "\n",
    )
    .with_context(|| format!("无法写入 {INIT_RECORD}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_validation() {
        assert!(valid_crate_name("my-cli"));
        assert!(valid_crate_name("a"));
        assert!(valid_crate_name("tool_2"));
        assert!(!valid_crate_name("My-cli"));
        assert!(!valid_crate_name("2fast"));
        assert!(!valid_crate_name("my cli"));
        assert!(!valid_crate_name(""));
    }

    #[test]
    fn crate_name_rejects_init_bin_collision() {
        // init 与 src/bin/init.rs 重名 → duplicate binary，构建失败
        assert!(!usable_crate_name("init"));
        assert!(usable_crate_name("my-cli"));
    }

    #[test]
    fn parse_repo_ref_formats() {
        assert_eq!(
            parse_repo_ref("https://github.com/alice/tool.git"),
            Some(("alice".to_string(), "tool".to_string()))
        );
        assert_eq!(
            parse_repo_ref("alice/tool"),
            Some(("alice".to_string(), "tool".to_string()))
        );
        assert_eq!(
            parse_repo_ref("https://gitee.com/a/b"),
            Some(("a".to_string(), "b".to_string()))
        );
        assert_eq!(parse_repo_ref("https://github.com/alice"), None);
        assert_eq!(parse_repo_ref("git@github.com:alice/tool"), None);
        assert_eq!(parse_repo_ref(""), None);
    }

    /// 造一棵迷你模板树：README/Cargo.toml/LICENSE/update.rs + 应被忽略的 target
    fn make_tree(dir: &Path) {
        fs::create_dir_all(dir.join("src/cli/commands")).unwrap();
        fs::create_dir_all(dir.join("target")).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"my-cli\"\ndefault-run = \"my-cli\"\nversion = \"0.1.0\"\ndescription = \"{{APP_DESCRIPTION}}\"\nauthors = [\"{{AUTHOR}}\"]\n",
        )
        .unwrap();
        fs::write(
            dir.join("README.md"),
            "# {{APP_NAME}}\n{{APP_DESCRIPTION}}\n",
        )
        .unwrap();
        fs::write(dir.join("LICENSE"), "Copyright (c) 2026 {{AUTHOR}}\n").unwrap();
        fs::write(
            dir.join("src/cli/commands/update.rs"),
            "pub const REPO_OWNER: &str = \"{{REPO_OWNER}}\";\n",
        )
        .unwrap();
        fs::write(dir.join("target/README.md"), "{{APP_NAME}}\n").unwrap();
        fs::write(dir.join("docs/guide.md"), "{{APP_NAME}}\n").unwrap();
        fs::write(
            dir.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"stable\"\n",
        )
        .unwrap();
    }

    fn values(name: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("APP_NAME".to_string(), name.to_string()),
            ("APP_DESCRIPTION".to_string(), "测试工具".to_string()),
            ("AUTHOR".to_string(), "张三".to_string()),
            ("REPO_OWNER".to_string(), "alice".to_string()),
            ("REPO_NAME".to_string(), "tool".to_string()),
        ])
    }

    #[test]
    fn apply_replaces_placeholders_and_skips_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        make_tree(tmp.path());
        apply(tmp.path(), &values("my-tool"), &InitRecord::default()).unwrap();

        assert_eq!(
            fs::read_to_string(tmp.path().join("README.md")).unwrap(),
            "# my-tool\n测试工具\n"
        );
        assert_eq!(
            fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap(),
            "[package]\nname = \"my-tool\"\ndefault-run = \"my-tool\"\nversion = \"0.1.0\"\ndescription = \"测试工具\"\nauthors = [\"张三\"]\n"
        );
        assert!(fs::read_to_string(tmp.path().join("LICENSE"))
            .unwrap()
            .contains("张三"));
        assert!(
            fs::read_to_string(tmp.path().join("src/cli/commands/update.rs"))
                .unwrap()
                .contains("\"alice\"")
        );
        // target 与 docs 不替换
        assert_eq!(
            fs::read_to_string(tmp.path().join("target/README.md")).unwrap(),
            "{{APP_NAME}}\n"
        );
        assert_eq!(
            fs::read_to_string(tmp.path().join("docs/guide.md")).unwrap(),
            "{{APP_NAME}}\n"
        );
    }

    #[test]
    fn apply_is_idempotent_with_old_values() {
        let tmp = tempfile::tempdir().unwrap();
        make_tree(tmp.path());

        // 第一次：my-tool
        let first = values("my-tool");
        apply(tmp.path(), &first, &InitRecord::default()).unwrap();
        // 第二次：改名 my-app、改仓库名 tool→renamed（携带上次记录，旧值一并替换）
        let mut second = values("my-app");
        second.insert("AUTHOR".to_string(), "李四".to_string());
        second.insert("REPO_NAME".to_string(), "renamed".to_string());
        let old_record = InitRecord(first.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        apply(tmp.path(), &second, &old_record).unwrap();

        let readme = fs::read_to_string(tmp.path().join("README.md")).unwrap();
        assert!(readme.contains("my-app"));
        assert!(!readme.contains("my-tool"));
        let cargo = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("name = \"my-app\""));
        assert!(cargo.contains("default-run = \"my-app\""));
        assert!(fs::read_to_string(tmp.path().join("LICENSE"))
            .unwrap()
            .contains("李四"));
        // rust-toolchain.toml 原样未变：若未被 SKIP_FILES 排除，
        // REPO_NAME tool→renamed 的旧值替换会腐蚀 [toolchain]
        assert_eq!(
            fs::read_to_string(tmp.path().join("rust-toolchain.toml")).unwrap(),
            "[toolchain]\nchannel = \"stable\"\n"
        );
    }

    #[test]
    fn apply_escapes_toml_values_only_in_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        make_tree(tmp.path());
        let mut v = values("my-tool");
        v.insert(
            "APP_DESCRIPTION".to_string(),
            "带\"引号\"与\\反斜杠".to_string(),
        );
        apply(tmp.path(), &v, &InitRecord::default()).unwrap();

        // Cargo.toml：`"`/`\` 转义为 `\"`/`\\`，产物是合法 TOML
        let cargo = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("带\\\"引号\\\"与\\\\反斜杠"));
        toml::from_str::<toml::Value>(&cargo).expect("Cargo.toml 应为合法 TOML");
        // README 等其他文件保留原字符
        assert_eq!(
            fs::read_to_string(tmp.path().join("README.md")).unwrap(),
            "# my-tool\n带\"引号\"与\\反斜杠\n"
        );
    }

    #[test]
    fn apply_replaces_escaped_old_value_in_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        make_tree(tmp.path());
        // 第一次：description 含引号（Cargo.toml 中以转义形态落盘）
        let mut first = values("my-tool");
        first.insert("APP_DESCRIPTION".to_string(), "带\"引号\"".to_string());
        apply(tmp.path(), &first, &InitRecord::default()).unwrap();
        // 第二次：改描述（携带上次记录，旧值也按转义形态匹配替换）
        let old_record = InitRecord(first.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let mut second = values("my-tool");
        second.insert("APP_DESCRIPTION".to_string(), "新描述".to_string());
        apply(tmp.path(), &second, &old_record).unwrap();

        let cargo = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("新描述"));
        assert!(!cargo.contains("引号"));
        toml::from_str::<toml::Value>(&cargo).expect("Cargo.toml 应为合法 TOML");
    }
}
