# 飞梭同步

<div align="center">

**一个用 Rust 编写的跨平台文件同步工具，适合开发目录、NAS、服务器和 Homelab 场景。**

[![Rust](https://img.shields.io/badge/Rust-Stable-orange.svg)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Runtime-Tokio-blue.svg)](https://tokio.rs/)
[![SQLite](https://img.shields.io/badge/Database-SQLite-green.svg)](https://www.sqlite.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[快速开始](#快速开始) | [核心能力](#核心能力) | [桌面端](#桌面端) | [命令行](#命令行) | [开发与发布](#开发与发布)

</div>

## 项目定位

飞梭同步把文件同步拆成两个角色：

- `turbosync-agent`：本机后台同步服务，负责扫描、监听、同步和传输。
- `tsync` / `tsync-gui`：用户入口，负责创建节点、创建任务、触发同步和查看状态。

日常使用优先推荐桌面 GUI 或 TUI 控制台。普通用户不需要理解后台服务细节，GUI/TUI 会自动处理本机 Agent 的启动和关闭。

## 核心能力

| 能力 | 当前状态 |
|------|----------|
| 桌面 GUI | 已支持，Tauri + React 桌面控制台，复用 Rust Agent 能力 |
| 终端 TUI | 已支持，默认中文，可切换 English |
| CLI | 已支持节点、任务、同步、扫描、监听、日志 |
| 本地同步 | 已支持本机目录到本机目录 |
| 远端同步 | 已支持 Agent-to-Agent QUIC 文件传输 |
| 文件监听 | 已支持 500ms 防抖，文件变化后自动扫描和同步 |
| 同步日志 | 已支持运行记录、变更数量、失败数量、传输数据量 |
| 节点健康检查 | 已支持连接成功、连接失败和错误信息 |
| 双向同步 | 已提供基础验证路径，适合早期测试 |
| 三平台打包 | 已配置 GitHub Actions，支持 Windows / Linux / macOS |

当前最稳定的主线是 **单向同步：源目录 -> 目标目录**。双向同步已有基础能力和自测流程，建议先在测试目录验证后再用于真实数据。

## 快速开始

### 方式一：桌面 GUI

```bash
make gui
```

`make gui` 会先构建本机 Agent 和 CLI，再启动桌面端。

GUI 启动后会自动初始化本机配置，并自动启动本机 Agent。如果这个 Agent 是 GUI 自己启动的，关闭 GUI 时会自动关闭它；如果 Agent 原本已经在运行，GUI 只连接使用，不会在退出时关闭它。

桌面端支持：

- 添加同步节点
- 创建同步任务
- 手动同步和重新扫描
- 开启或停止文件监听
- 查看节点健康状态
- 查看最近同步活动
- 删除任务和节点

### 方式二：终端控制台

```bash
tsync dashboard
```

第一次运行时，`dashboard` 会自动初始化本机状态，并启动本机 Agent。这个 Agent 如果是 dashboard 自动启动的，退出控制台时会自动关闭；如果本来已经在运行，dashboard 不会关闭它。

常用按键：

| 按键 | 作用 |
|------|------|
| `s` | 同步选中任务 |
| `w` | 开启文件监听 |
| `x` | 停止文件监听 |
| `e` | 编辑任务 |
| `f` | 查看文件索引 |
| `r` | 刷新 |
| `l` | 中文 / English 切换 |
| `?` | 帮助 |
| `q` | 退出 |

## 桌面端

桌面端二进制名是 `tsync-gui`，产品名显示为 **飞梭同步**。界面使用 Tauri + React + TypeScript 实现，保留 Rust 后端同步能力，并提供更现代的桌面控制台体验。

发布包中会同时包含：

- `tsync-gui`
- `tsync`
- `turbosync-agent`

GUI 会按顺序寻找可启动的本机 Agent：

1. GUI 同目录下的 `turbosync-agent`
2. GUI 同目录下的 `tsync agent run`
3. 源码仓库 `target/debug` 或 `target/release` 下的本机二进制
4. `PATH` 中的 `turbosync-agent` 或 `tsync`

这保证了发布包、源码开发和系统安装三种方式都能运行。

## 命令行

### 初始化

通常不需要手动执行，GUI/TUI 会自动初始化。

```bash
tsync init
```

### 前台运行 Agent

用于远端机器、Docker、服务器或调试日志：

```bash
tsync agent run
```

### 添加节点

本机添加远端 Agent：

```bash
tsync node add nas 192.168.1.20:38746
tsync node list
tsync node remove <node-id>
```

如果远端启用了证书指纹校验：

```bash
tsync node add nas 192.168.1.20:38746 --cert-fingerprint <sha256>
```

### 创建同步任务

```bash
tsync task add documents \
  --source ~/Documents \
  --target-node <node-id> \
  --target-path /backup/Documents
```

任务管理：

```bash
tsync task list
tsync task remove <task-id>
```

### 同步、扫描、监听和日志

```bash
tsync sync <task-id>
tsync rescan <task-id>
tsync watch <task-id> start
tsync watch <task-id> status
tsync watch <task-id> stop
tsync logs --limit 20
```

## Docker / 远端机器

远端机器只需要运行 Agent，并暴露控制和传输端口。

示例：

```bash
tsync agent run
```

Docker 示例：

```bash
docker run -it \
  --name ubuntu-01 \
  -p 127.0.0.1:38750:38745 \
  -d \
  ubuntu:24.04
```

容器内安装或复制 TurboSync 后运行：

```bash
tsync agent run
```

本机添加节点时使用映射出来的地址：

```bash
tsync node add docker-ubuntu 127.0.0.1:38750
```

## 架构

```text
Desktop GUI / TUI Dashboard / CLI
        |
        | localhost HTTP
        v
TurboSync Agent
  |-- Local Control API (axum)
  |-- SQLite State DB
  |-- File Watcher (notify)
  |-- Scanner / Indexer (walkdir + blake3)
  |-- Sync Engine
  |-- QUIC Transport (quinn)
```

工作流：

1. GUI/TUI/CLI 调用本机 Agent API。
2. Agent 扫描源目录，写入 SQLite 文件索引。
3. 同步引擎生成创建、更新、删除计划。
4. 本地目标直接复制文件。
5. 远端目标通过 QUIC 发送到目标 Agent。
6. 同步结果写入日志，并展示在 GUI/TUI/CLI 中。

## Workspace

```text
turbo_sync/
|-- crates/
|   |-- turbosync-agent/      # 本机 Agent 和控制 API
|   |-- turbosync-cli/        # tsync 命令行入口
|   |-- turbosync-core/       # 配置、模型、通用类型
|   |-- turbosync-gui/        # Tauri + React 桌面 GUI，独立应用
|   |-- turbosync-storage/    # SQLite / SQLx 持久化
|   |-- turbosync-sync/       # 文件扫描、同步引擎、文件监听
|   |-- turbosync-transport/  # Agent-to-Agent QUIC 传输
|   `-- turbosync-tui/        # 终端仪表盘
|-- migrations/               # SQLx migrations
|-- docs/                     # 用户指南、自测流程、产品文案
|-- Makefile
`-- README.md
```

## 开发与发布

### 本地检查

```bash
make check
```

等价于：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

GUI 是 Tauri + React 独立应用，需要单独检查：

```bash
make build-gui
make gui-check
```

等价于前端构建、Tauri Rust fmt/clippy/test 和 Tauri shell 编译检查。

### 手动自测

详细流程见：

```text
docs/testing.zh.md
```

覆盖：

- 本地 -> 本地
- 本地 -> 远端
- 双向同步基础验证
- TUI 快速验收

### GitHub Actions

CI 已配置：

- Rust workspace 格式检查
- Rust workspace clippy
- Rust workspace tests
- Desktop GUI build/check
- Desktop GUI fmt/clippy

Release workflow 会在 `v*` tag 推送时构建三平台包：

- Linux x64
- macOS x64
- macOS ARM64
- Windows x64

创建发布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

## 文档

```text
docs/user-guide.zh.md          # 用户使用指南，TUI 优先
docs/testing.zh.md             # 开发和发布前自测流程
docs/product/landing.zh.md     # 产品介绍和官网文案
```

## 适用场景

- 开发目录在多台机器之间同步
- 本机项目目录同步到 NAS
- Linux 服务器和本地工作站之间同步
- Docker / Homelab 环境里的目录传输
- 想用 TUI 或 GUI 管理同步任务，而不是手写复杂命令

## 当前边界

- 推荐先使用单向同步处理真实数据。
- 双向同步仍建议在测试目录中验证后再使用。
- 远端同步建议在可信网络中运行，并配置证书指纹。
- 大规模团队权限、云盘协作、在线预览不是当前版本目标。

## License

MIT
