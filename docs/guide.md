# 如何添加你的第一个子系统

以模板自带的 **demo 域**为参照，新增一个子系统共六步。假设新域叫 `note`。

## 第 1 步：命令树（args.rs）

`src/cli/args.rs` 的 `Command` 枚举加变体：

```rust
/// 笔记管理（示例新域）
Note {
    #[command(subcommand)]
    command: NoteCommand,
},
```

同文件新增 `NoteCommand` 枚举（每个子命令一个变体，含参数与文档注释）。

## 第 2 步：分发（main.rs）

`src/main.rs` 的 match 加分支 `Command::Note { command } => dispatch_note(command)?`，
并新增 `dispatch_note` 函数解包参数调用 `cli::commands::note::<子命令>::execute(...)`。

## 第 3 步：命令实现层（cli/commands/note/）

`src/cli/commands/note/mod.rs` 声明子命令模块；每个子命令一个文件：
参数解包 → `Settings::load()` → 客户端调用 → formatter 输出。
参照 `cli/commands/demo/repos.rs` 的三格式输出写法。

## 第 4 步：业务客户端层（src/note/）

- `client.rs` - HTTP 客户端（参照 `demo/client.rs`：from_settings 校验配置、
  check 状态码分层、decode 反序列化）
- `types.rs` - API 类型映射（serde rename/default，参照 `demo/types.rs`）
- `error.rs` - thiserror 错误分层（参照 `demo/error.rs`：Request/Status/Decode）

`src/main.rs` 加 `mod note;`。

## 第 5 步：配置 section（config/settings.rs）

`Settings` 加字段 `#[serde(default)] pub note: NoteSettings`，实现 Default、
在 `set()` 与 `display()` 各加一支（敏感字段用 `mask()` 脱敏）。
命令层加 `cli/commands/note/config.rs`（参照 demo 的 config，见第 3 步）。

## 第 6 步：doctor 检测 + 文档

- `src/cli/commands/doctor.rs` 加一项检测（参照 check_api：未配置跳过、
  连通 ✓、失败带 network_hint）
- README 的"配置"与命令示例补新域；本文件不动

完成。删域 = 反向删除这六步产物。
