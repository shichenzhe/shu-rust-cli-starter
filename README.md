# 枢 · {{APP_NAME}}

> 枢者，门轴也——门之开合，皆系于枢。一切命令行工具，由此开启。

开箱即用的 **Rust 多命令 CLI 脚手架**：clap 命令树、统一 TOML 配置、doctor
连通性检测、装机/卸载、GitHub Releases 自更新、三平台 CI/发布，一个模板全都有。

## 特性

- **多命令骨架**：clap derive 命令树 + 分发（参照 `demo` 示范域扩展你的子系统）
- **统一配置**：`~/.<app>/config.toml`；交互式 `config init`（token 不回显）、
  就近 `demo config set/show`（token 脱敏展示）
- **doctor 连通性检测**：配置合法性 + API 可达性，逐项 ✓/✗/跳过，失败退出码 1
- **装机/卸载**：`install` 复制到 `~/.local/bin`（Windows 写注册表 PATH）
- **自更新**：`update check/apply`，走 GitHub Releases 制品
- **表格/CSV/JSON 输出**：CJK 对齐的通用 formatter
- **三平台 CI**：macOS / Linux / Windows（fmt + clippy -D warnings + test + build）
- **一键发布**：push tag `v*` 自动构建三平台四份制品并挂 Release

## 技术栈

Rust 2021 · clap 4.6 · reqwest 0.13（native-tls）+ tokio · serde / toml · anyhow / thiserror ·
tracing · self_update · assert_cmd

## 快速开始

```bash
# 1. 用本模板创建仓库（GitHub "Use this template" 或）
npx degit <your-fork>/shu-rust-cli-starter my-cli
cd my-cli

# 2. 初始化（应用名 / 描述 / 作者 / 仓库地址，可重复运行）
cargo run --bin init

# 3. 构建与测试
cargo build
cargo test -- --test-threads=1

# 4. 试试示范域（GitHub 公开 API，无需 token）
cargo run -- demo repos torvalds
cargo run -- demo issues rust-lang/rust
```

> `cargo run --bin init` 应在第一次构建前运行——它替换 Cargo.toml 的 crate 名与
> 文档占位符。不 init 也能构建（默认名 `my-cli`），但二进制名与配置目录都是默认值。
> 输入记录在 `starter-init.json`（已 gitignore），重复运行幂等。

## 目录结构

```
├── src/
│   ├── bin/init.rs         # 交互式初始化（占位符替换，幂等）
│   ├── main.rs             # 入口：tracing→stderr + 命令分发
│   ├── cli/args.rs         # clap 命令树
│   ├── cli/commands/       # 命令实现（demo/{repos,issues,config} + config/doctor/setup/update）
│   ├── config/settings.rs  # 统一 TOML 配置
│   ├── demo/               # 示范域业务客户端层（client/types/error）
│   └── output/formatter.rs # 表格（CJK 对齐）/CSV/JSON
├── tests/                  # 集成测试（assert_cmd；HOME 改写需 --test-threads=1）
└── .github/workflows/      # ci.yml / release.yml
```

**添加你的第一个子系统** → [docs/guide.md](docs/guide.md)

## 配置

所有子系统共用 `~/.<app>/config.toml`：

```bash
# 交互式初始化（token 不回显；已存在则回车保留原值）
<app> config init

# demo 域就近配置（key 不带前缀）
<app> demo config init
<app> demo config set base_url https://api.github.com   # 默认值
<app> demo config set token <GITHUB_TOKEN>              # 可选
<app> demo config show                                  # token 脱敏
```

配置文件示例：

```toml
[demo]
base_url = "https://api.github.com"
token = ""
```

## 安装 / 卸载 / 更新

```bash
<app> install            # 复制到 ~/.local/bin（Windows: %LOCALAPPDATA%\<app>\）
<app> uninstall
<app> doctor             # 检测配置与连通性，失败退出码 1
<app> update check       # 检查新版本（GitHub Releases）
<app> update apply       # 更新到最新版本
```

## 发布

1. bump `Cargo.toml` 的 `version`（semver）
2. `git tag vX.Y.Z && git push --tags`
3. release.yml 自动构建三平台四份制品（macOS ARM/x86_64、Linux x86_64、Windows x86_64），
   归档根为可执行文件、命名含 target triple，挂到 Release assets
4. 用户侧 `<app> update check` 即可发现新版本

## License

[MIT](LICENSE)
