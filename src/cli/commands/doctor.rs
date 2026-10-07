use crate::config::settings::Settings;
use crate::demo::client::DemoClient;
use anyhow::Result;

/// 单项检测结果
struct Check {
    name: &'static str,
    target: String,
    outcome: Outcome,
}

enum Outcome {
    Pass(String),
    Fail(String),
    Skip(String),
}

impl Check {
    fn new(name: &'static str, target: String, outcome: Outcome) -> Self {
        Self {
            name,
            target,
            outcome,
        }
    }
}

/// 执行 doctor：检测配置文件与 API 连通性，失败项给出说明
pub fn execute() -> Result<()> {
    println!(
        "{} doctor - 检测配置与 API 连通性\n",
        env!("CARGO_BIN_NAME")
    );

    let mut checks: Vec<Check> = Vec::new();

    // ---- 1. 配置文件 ----
    let config_path = Settings::config_path()?;
    let settings = if !config_path.exists() {
        checks.push(Check::new(
            "配置文件",
            config_path.display().to_string(),
            Outcome::Fail(format!(
                "文件不存在，请先运行 {} config init",
                env!("CARGO_BIN_NAME")
            )),
        ));
        None
    } else {
        match Settings::load() {
            Ok(s) => {
                checks.push(Check::new(
                    "配置文件",
                    config_path.display().to_string(),
                    Outcome::Pass("TOML 格式正确".to_string()),
                ));
                Some(s)
            }
            Err(e) => {
                checks.push(Check::new(
                    "配置文件",
                    config_path.display().to_string(),
                    Outcome::Fail(format!("解析失败：{e:#}")),
                ));
                None
            }
        }
    };

    // ---- 2/3. demo API 连通 + token 有效性 ----
    // 配置不可用时无法构建客户端，后续项统一说明跳过原因
    let rt = tokio::runtime::Runtime::new()?;
    let unavailable = "配置文件不可用，先修复后再检测";
    checks.push(match &settings {
        Some(s) => check_api(s, &rt),
        None => Check::new(
            "demo API",
            "-".to_string(),
            Outcome::Skip(unavailable.to_string()),
        ),
    });
    checks.push(match &settings {
        Some(s) => check_token(s, &rt),
        None => Check::new(
            "token",
            "-".to_string(),
            Outcome::Skip(unavailable.to_string()),
        ),
    });

    // ---- 输出 ----
    let total = checks.len();
    let (mut pass, mut fail, mut skip) = (0usize, 0usize, 0usize);
    for (i, c) in checks.iter().enumerate() {
        println!("[{}/{}] {}（{}）", i + 1, total, c.name, c.target);
        match &c.outcome {
            Outcome::Pass(msg) => {
                println!("      ✓ {msg}");
                pass += 1;
            }
            Outcome::Fail(msg) => {
                println!("      ✗ {msg}");
                fail += 1;
            }
            Outcome::Skip(msg) => {
                println!("      - {msg}（跳过）");
                skip += 1;
            }
        }
    }

    println!("\n检测完成：{pass} 通过，{fail} 失败，{skip} 跳过");
    if fail > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// demo API：GET /zen 验证 base_url 可达
fn check_api(settings: &Settings, rt: &tokio::runtime::Runtime) -> Check {
    let client = match DemoClient::from_settings(settings) {
        Ok(c) => c,
        Err(e) => {
            return Check::new(
                "demo API",
                settings.demo.base_url.clone(),
                Outcome::Fail(format!("{e:#}")),
            )
        }
    };
    match rt.block_on(client.ping()) {
        Ok(zen) => Check::new(
            "demo API",
            settings.demo.base_url.clone(),
            Outcome::Pass(format!("连通正常（{zen}）")),
        ),
        Err(e) => Check::new(
            "demo API",
            settings.demo.base_url.clone(),
            Outcome::Fail(format!("{e:#}{}", network_hint(&e))),
        ),
    }
}

/// token：GET /user 验证有效性（未配置则跳过）
fn check_token(settings: &Settings, rt: &tokio::runtime::Runtime) -> Check {
    if settings.demo.token.is_empty() {
        return Check::new(
            "token",
            "-".to_string(),
            Outcome::Skip("未配置（匿名调用限 60 次/小时，仅影响速率）".to_string()),
        );
    }
    let client = match DemoClient::from_settings(settings) {
        Ok(c) => c,
        Err(e) => return Check::new("token", "-".to_string(), Outcome::Fail(format!("{e:#}"))),
    };
    match rt.block_on(client.whoami()) {
        Ok(login) => Check::new(
            "token",
            "-".to_string(),
            Outcome::Pass(format!("认证通过（{login}）")),
        ),
        Err(e) => Check::new(
            "token",
            "-".to_string(),
            Outcome::Fail(format!("{e:#}{}", network_hint(&e))),
        ),
    }
}

/// 网络类错误的补充提示（连接失败/超时给出排查方向）
fn network_hint(e: &anyhow::Error) -> String {
    let hint = e
        .chain()
        .find_map(|c| {
            c.downcast_ref::<reqwest::Error>().and_then(|re| {
                if re.is_connect() {
                    Some("（无法连接：检查网络或 base_url 是否可达）")
                } else if re.is_timeout() {
                    Some("（请求超时）")
                } else {
                    None
                }
            })
        })
        .unwrap_or("");
    hint.to_string()
}
