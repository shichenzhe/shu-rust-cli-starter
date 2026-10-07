use serde::{Deserialize, Serialize};

/// 仓库摘要（GET /users/{owner}/repos 元素的精简映射）
#[derive(Debug, Serialize, Deserialize)]
pub struct RepoSummary {
    pub name: String,
    #[serde(rename = "stargazers_count")]
    pub stars: u64,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// issue 摘要（对外输出结构；由 RawIssue 转换，PR 已过滤）
#[derive(Debug, Serialize, Deserialize)]
pub struct IssueSummary {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// issues 接口原始结构（PR 也会混在其中）
#[derive(Debug, Deserialize)]
pub struct RawIssue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub user: Option<RawUser>,
    #[serde(default)]
    pub labels: Vec<RawLabel>,
    /// 该字段存在即为 PR，需过滤
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawUser {
    #[serde(default)]
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct RawLabel {
    #[serde(default)]
    pub name: String,
}

impl From<RawIssue> for IssueSummary {
    fn from(raw: RawIssue) -> Self {
        Self {
            number: raw.number,
            title: raw.title,
            author: raw.user.map(|u| u.login),
            labels: raw.labels.into_iter().map(|l| l.name).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_repo_summary() {
        let json = serde_json::json!({
            "name": "linux",
            "stargazers_count": 180000,
            "language": "C",
            "description": "kernel"
        });
        let r: RepoSummary = serde_json::from_value(json).unwrap();
        assert_eq!(r.name, "linux");
        assert_eq!(r.stars, 180000);
        assert_eq!(r.language.as_deref(), Some("C"));
    }

    #[test]
    fn parse_issue_and_convert() {
        let json = serde_json::json!({
            "number": 7,
            "title": "修复",
            "user": {"login": "alice"},
            "labels": [{"name": "bug"}, {"name": "help wanted"}]
        });
        let raw: RawIssue = serde_json::from_value(json).unwrap();
        assert!(raw.pull_request.is_none());
        let s = IssueSummary::from(raw);
        assert_eq!(s.number, 7);
        assert_eq!(s.author.as_deref(), Some("alice"));
        assert_eq!(s.labels, vec!["bug", "help wanted"]);
    }

    #[test]
    fn pr_has_pull_request_field() {
        let json = serde_json::json!({
            "number": 9,
            "title": "PR",
            "pull_request": {"url": "https://api.github.com/..."}
        });
        let raw: RawIssue = serde_json::from_value(json).unwrap();
        assert!(raw.pull_request.is_some());
    }
}
