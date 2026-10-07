# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

`{{APP_NAME}}`——Rust 多命令 CLI 脚手架（从 shu-rust-cli-starter 模板创建）。
示范域 `demo` 调用 GitHub 公开 API。

## 开发命令

```bash
cargo build                                     # 构建
cargo test -- --test-threads=1                  # 测试（HOME 改写必须串行）
cargo fmt --all                                 # 格式化
cargo clippy --all-targets -- -D warnings       # lint
cargo run -- demo repos torvalds                # 试运行示范域（真实 API）
cargo run --bin init                            # 模板初始化（一般勿在模板仓库执行）
```

**集成测试改写 HOME 环境变量隔离配置目录，必须 `--test-threads=1` 串行执行。**

## 项目架构

- `src/main.rs` - 入口：tracing 初始化（统一 stderr，stdout 是数据通道）+ 命令分发
- `src/cli/args.rs` - clap derive 命令树（Command → DemoCommand / Config / Doctor / Install / Uninstall / Update）
- `src/cli/commands/` - 命令实现层（按子系统就近组织：`demo/` + `config/doctor/setup/update.rs`）
- `src/config/settings.rs` - 统一 TOML 配置（`~/.<crate-name>/config.toml`，load/save/set/display 脱敏）
- `src/demo/` - 示范域业务客户端层（`client.rs` async reqwest + `types.rs` serde 映射 + `error.rs` thiserror 分层）
- `src/output/formatter.rs` - 表格（CJK 双宽对齐）/ CSV / JSON 输出
- `src/bin/init.rs` - 模板初始化（占位符替换，幂等）
- `tests/integration_test.rs` - assert_cmd 集成测试

## 代码约定

- 中文注释
- 错误处理：anyhow（应用层）+ thiserror（库层，参照 `src/demo/error.rs` 分层范式）
- 日志：tracing，只写 stderr（stdout 是命令数据通道）
- 新子系统参照 `docs/guide.md` 六步法（以 demo 域为模板）

## 脚手架专有

本仓库是脚手架模板，内置 5 个占位符（APP_NAME / APP_DESCRIPTION / AUTHOR /
REPO_OWNER / REPO_NAME，即 `src/bin/init.rs` 的 PLACEHOLDERS 常量），由
`cargo run --bin init` 交互式替换（幂等，输入记录在 `starter-init.json`）。

- **禁止在 .rs 中硬编码 bin 名/应用名**：一律 `env!("CARGO_BIN_NAME")` /
  `env!("CARGO_PKG_NAME")`——init 改名后源码零改动
- **唯一例外**：`src/cli/commands/update.rs` 的 `REPO_OWNER`/`REPO_NAME` 常量
  （self_update 需要编译期常量，无 env!() 通道），init 按文件白名单定点替换；
  `LICENSE`（无扩展名）同为定点替换
- Cargo.toml 的 `name` 用合法默认值 `my-cli`（保证模板自身 CI 可构建），init 定点替换
- init 扫描 `.toml/.md/.yml` + 定点上述两文件，忽略 `target/.git/.github/docs`
- 发布约定：归档根直接是可执行文件、制品命名含 target triple（self_update 匹配依赖），
  见 `.github/workflows/release.yml`
