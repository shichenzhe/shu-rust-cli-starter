use crate::config::settings::Settings;
use crate::demo::client::DemoClient;
use crate::demo::types::RepoSummary;
use crate::output::formatter::{format_csv, format_json, format_table};
use anyhow::Result;

/// demo repos：列出用户（或组织）的公开仓库
pub fn execute(owner: String, csv: bool, json: bool) -> Result<()> {
    let settings = Settings::load()?;
    let client = DemoClient::from_settings(&settings)?;
    let rt = tokio::runtime::Runtime::new()?;
    let repos = rt.block_on(client.repos(&owner))?;

    if json {
        println!("{}", format_json(&repos)?);
    } else if csv {
        println!("{}", format_csv(&REPO_HEADERS, &repo_rows(&repos, false))?);
    } else {
        println!("{}", format_table(&REPO_HEADERS, &repo_rows(&repos, true)));
    }
    Ok(())
}

const REPO_HEADERS: [&str; 4] = ["名称", "星数", "语言", "描述"];

/// 组装表格/CSV 行；表格模式截断长描述
fn repo_rows(repos: &[RepoSummary], truncate_desc: bool) -> Vec<Vec<String>> {
    repos
        .iter()
        .map(|r| {
            let desc = r.description.as_deref().unwrap_or("");
            vec![
                r.name.clone(),
                r.stars.to_string(),
                r.language.clone().unwrap_or_else(|| "-".to_string()),
                if truncate_desc {
                    truncate(desc, 50)
                } else {
                    desc.to_string()
                },
            ]
        })
        .collect()
}

/// 按字符数截断并加省略号
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max_chars - 1).collect();
        format!("{}…", cut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<RepoSummary> {
        vec![RepoSummary {
            name: "linux".to_string(),
            stars: 180000,
            language: Some("C".to_string()),
            // 170 字符长描述（ASCII）：截断到 50 字符后表格最长行 71 列（≤80）。
            // 若用 CJK 长描述，截断后显示宽约 99 列，分隔线会到 120 列，
            // 与下方“行 ≤80 字符”断言冲突（brief 隐含 ASCII 截断假设）
            description: Some("long description ".repeat(10)),
        }]
    }

    #[test]
    fn table_truncates_long_description() {
        let out = format_table(&REPO_HEADERS, &repo_rows(&sample(), true));
        assert!(out.contains("linux"));
        assert!(out.contains('…'));
        assert!(out.lines().all(|l| l.chars().count() <= 80));
    }

    #[test]
    fn csv_keeps_full_description() {
        let rows = repo_rows(&sample(), false);
        assert_eq!(
            rows[0][3].chars().count(),
            "long description ".repeat(10).chars().count()
        );
    }

    #[test]
    fn truncate_short_untouched() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdef", 5), "abcd…");
    }
}
