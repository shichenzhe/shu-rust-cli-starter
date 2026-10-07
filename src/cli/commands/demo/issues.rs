use crate::config::settings::Settings;
use crate::demo::client::DemoClient;
use crate::demo::types::IssueSummary;
use crate::output::formatter::{format_csv, format_json, format_table};
use anyhow::{Context, Result};

/// demo issues：列出仓库的 open issues（过滤 PR）
pub fn execute(repo: String, csv: bool, json: bool) -> Result<()> {
    let (owner, name) = repo
        .split_once('/')
        .context("仓库格式应为 owner/repo（如 torvalds/linux）")?;

    let settings = Settings::load()?;
    let client = DemoClient::from_settings(&settings)?;
    let rt = tokio::runtime::Runtime::new()?;
    let issues = rt.block_on(client.issues(owner, name))?;

    if json {
        println!("{}", format_json(&issues)?);
    } else if csv {
        println!("{}", format_csv(&ISSUE_HEADERS, &issue_rows(&issues))?);
    } else {
        println!("{}", format_table(&ISSUE_HEADERS, &issue_rows(&issues)));
    }
    Ok(())
}

const ISSUE_HEADERS: [&str; 4] = ["编号", "标题", "作者", "标签"];

fn issue_rows(issues: &[IssueSummary]) -> Vec<Vec<String>> {
    issues
        .iter()
        .map(|i| {
            vec![
                i.number.to_string(),
                i.title.clone(),
                i.author.clone().unwrap_or_else(|| "-".to_string()),
                i.labels.join(", "),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_join_labels() {
        let issues = vec![IssueSummary {
            number: 7,
            title: "修复".to_string(),
            author: Some("alice".to_string()),
            labels: vec!["bug".to_string(), "P1".to_string()],
        }];
        let rows = issue_rows(&issues);
        assert_eq!(rows[0][0], "7");
        assert_eq!(rows[0][3], "bug, P1");
    }
}
