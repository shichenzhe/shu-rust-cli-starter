use crate::config::settings::Settings;
use crate::demo::error::DemoApiError;
use crate::demo::types::{IssueSummary, RawIssue, RepoSummary};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// GitHub API 客户端（demo 域示范：async reqwest + 可选 Bearer token）
pub struct DemoClient {
    http: reqwest::Client,
    base_url: String,
}

impl DemoClient {
    /// 从配置创建客户端（token 可选；UA 取编译期 bin 名）
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let base_url = settings.demo.base_url.trim_end_matches('/').to_string();
        if base_url.is_empty() {
            anyhow::bail!("demo base_url 未配置，请运行 demo config set base_url <URL>");
        }

        let mut headers = HeaderMap::new();
        // GitHub API 强制要求 User-Agent；init 改名后自动跟随
        let ua = env!("CARGO_BIN_NAME")
            .parse()
            .map_err(|_| anyhow!("UA 构造失败：bin 名含非法 header 字符"))?;
        headers.insert(USER_AGENT, ua);
        if !settings.demo.token.is_empty() {
            let auth = format!("Bearer {}", settings.demo.token)
                .parse()
                .map_err(|_| anyhow!("token 含非法字符，无法构造 Authorization header"))?;
            headers.insert(AUTHORIZATION, auth);
        }

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .context("无法创建 HTTP 客户端")?;
        Ok(Self { http, base_url })
    }

    /// 列出 owner 的公开仓库
    pub async fn repos(&self, owner: &str) -> Result<Vec<RepoSummary>> {
        let url = format!("{}/users/{}/repos?per_page=100", self.base_url, owner);
        let resp = self.http.get(&url).send().await?;
        let repos: Vec<RepoSummary> = self.decode(self.check(resp).await?).await?;
        Ok(repos)
    }

    /// 连通性探测：GET /zen（doctor 用）
    pub async fn ping(&self) -> Result<String> {
        let url = format!("{}/zen", self.base_url);
        let resp = self.http.get(&url).send().await?;
        Ok(self.check(resp).await?.text().await?)
    }

    /// token 有效性探测：GET /user（doctor 用；未配置 token 不应调用）
    pub async fn whoami(&self) -> Result<String> {
        let url = format!("{}/user", self.base_url);
        let resp = self.http.get(&url).send().await?;
        let value: Value = self.decode(self.check(resp).await?).await?;
        Ok(value["login"].as_str().unwrap_or("未知用户").to_string())
    }

    /// 列出 owner/repo 的 open issues（过滤混入的 PR）
    pub async fn issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueSummary>> {
        let url = format!(
            "{}/repos/{}/{}/issues?state=open&per_page=100",
            self.base_url, owner, repo
        );
        let resp = self.http.get(&url).send().await?;
        let raw: Vec<RawIssue> = self.decode(self.check(resp).await?).await?;
        Ok(raw
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .map(IssueSummary::from)
            .collect())
    }

    /// 校验响应状态码，常见错误给友好提示
    async fn check(&self, resp: reqwest::Response) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        let message = extract_message(&body);
        let err = match code {
            404 => DemoApiError::Status {
                status: code,
                message: format!("资源不存在（{message}）"),
            },
            401 => DemoApiError::Status {
                status: code,
                message: "认证失败：token 无效".to_string(),
            },
            403 if message.contains("rate limit") => DemoApiError::Status {
                status: code,
                message: "触发速率限制：匿名调用上限 60 次/小时，可配置 token 提升".to_string(),
            },
            _ => DemoApiError::Status {
                status: code,
                message,
            },
        };
        Err(err.into())
    }

    /// 响应体反序列化（reqwest 的 json 错误统一归到 Decode）
    async fn decode<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        resp.json::<T>()
            .await
            .map_err(|e| DemoApiError::Decode(e.to_string()).into())
    }
}

/// 从错误响应体提取 message 字段（GitHub 错误格式 {"message": "..."}）
fn extract_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_settings_rejects_empty_base_url() {
        let mut s = Settings::skeleton();
        s.demo.base_url = String::new();
        assert!(DemoClient::from_settings(&s).is_err());
    }

    #[test]
    fn from_settings_ok_with_default() {
        let s = Settings::skeleton();
        assert!(DemoClient::from_settings(&s).is_ok());
    }

    #[test]
    fn from_settings_rejects_token_with_illegal_chars() {
        // header value 禁止控制字符：parse 失败应报错而非 panic
        let mut s = Settings::skeleton();
        s.demo.token = "bad\ntoken".to_string();
        // DemoClient 未实现 Debug，用 err().unwrap() 取代 unwrap_err()
        let err = DemoClient::from_settings(&s).err().unwrap().to_string();
        assert!(err.contains("token 含非法字符"));
    }

    #[test]
    fn extract_message_prefers_message_field() {
        assert_eq!(extract_message(r#"{"message":"Not Found"}"#), "Not Found");
        assert_eq!(extract_message("plain text"), "plain text");
    }
}
