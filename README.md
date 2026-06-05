# 🚀 TurboSync

<div align="center">

**一个用 Rust 编写的高性能跨平台文件同步工具，面向开发者、NAS、Linux 和 Homelab 用户**

[![Rust](https://img.shields.io/badge/Rust-Stable-orange.svg)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Runtime-Tokio-blue.svg)](https://tokio.rs/)
[![SQLite](https://img.shields.io/badge/Database-SQLite-green.svg)](https://www.sqlite.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[✨ 核心目标](#-核心目标) • [🏗️ 架构设计](#️-架构设计) • [🚀 快速开始](#-快速开始) • [🛠️ 技术栈](#️-技术栈) • [📚 文档](#-文档)

</div>

## ✨ 核心目标

TurboSync 的目标是提供一个比 `rsync` 更简单、比 Syncthing 更容易上手的文件同步工具。

第一阶段 MVP 聚焦最小可用同步闭环：

<div align="center">

| 🧩 **CLI** | 🖥️ **Agent** | 👀 **文件监听** | 🔄 **文件同步** |
|:---:|:---:|:---:|:---:|
| 命令行管理节点和任务 | 本地常驻同步进程 | 监听目录变化 | 将源目录同步到目标节点 |

</div>

第一阶段暂不作为重点实现：

- 用户系统
- 权限系统
- 集群
- 云盘能力
- 在线预览
- 文档协作

桌面 GUI 已作为可选入口加入，Web 页面仅作为 Agent 内置调试/轻量控制页。

## 📌 当前状态

已经完成：

- ✅ Cargo workspace
- ✅ `tsync` CLI 骨架
- ✅ foreground Agent 骨架
- ✅ `tsync init`
- ✅ 平台标准配置目录
- ✅ TOML 配置文件
- ✅ SQLite 状态数据库
- ✅ SQLx migrations
- ✅ Agent 本地控制 API：health / status / nodes / tasks
- ✅ CLI 通过本地 Agent API 管理节点和同步任务
- ✅ 文件扫描和 SQLite 索引
- ✅ 本地手动同步：rescan / sync
- ✅ 同步 run / operation 日志
- ✅ 文件监听 + 500ms 防抖 (notify + tokio timer)
- ✅ Agent-to-Agent 文件传输 (QUIC / quinn)
- ✅ CLI watch 子命令 (start / stop / status)
- ✅ 终端仪表盘 (`tsync dashboard`, ratatui + crossterm)
- ✅ 桌面 GUI (`tsync-gui`, Slint，Windows / Linux / macOS)

## 🏗️ 架构设计

```text
CLI / TUI Dashboard / Desktop GUI
 ↓ localhost HTTP
TurboSync Agent
 ├─ Local Control API (axum)
 ├─ SQLite State DB
 ├─ File Watcher (notify, 500ms debounce)
 ├─ Scanner / Indexer (walkdir, blake3)
 ├─ Sync Engine (本地 + 远端路由)
 └─ QUIC Transport (quinn)
```

第一阶段采用 **source → target** 的单向同步模型，源目录是权威数据源。重命名可以先按“删除旧路径 + 新增新路径”处理，后续再扩展冲突处理和双向同步。

## 🚀 快速开始

### 1. 打开终端同步控制台

```bash
tsync dashboard
```

第一次运行时，`dashboard` 会自动初始化本地状态，并启动本地后台同步服务。这个服务如果是由 `dashboard` 自动启动的，退出控制台时会自动关闭；如果服务本来已经在运行，`dashboard` 不会关闭它。

这里的 Agent 指 TurboSync 的本地后台同步服务：它负责扫描文件、监听变化、执行同步。普通使用不需要手动管理它。

终端同步控制台是日常使用入口，默认显示中文，按 `l` 可切换 English。它围绕“源目录 → 目标目录”的同步链路组织信息，支持：

- 查看同步任务、后台服务状态、当前同步链路和最近活动
- 按 `s` 同步选中任务
- 按 `w` / `x` 开启或停止文件监听
- 按 `e` 编辑任务
- 按 `f` 查看文件索引
- 按 `l` 切换中英文
- 按 `?` 查看帮助

### 2. 打开桌面 GUI

桌面端是独立二进制 `tsync-gui`，连接本机 Agent API，不使用 Electron 或 WebView。

```bash
make gui
```

或直接运行：

```bash
cargo run --manifest-path crates/turbosync-gui/Cargo.toml
```

GUI 支持添加节点、添加任务、手动同步、扫描、监听开关、删除任务/节点、查看最近活动。双方机器仍然都需要启动 Agent；GUI 只是本机控制入口。

## 🧭 常用命令

### 管理节点

```bash
tsync node add nas 192.168.1.20:38746
tsync node list
tsync node remove <node-id>
```

### 管理同步任务

```bash
tsync task add documents \
  --source ~/Documents \
  --target-node <node-id> \
  --target-path /backup/Documents

tsync task list
tsync task remove <task-id>
```

### 文件监听

```bash
tsync watch <task-id> start
tsync watch <task-id> status
tsync watch <task-id> stop
```

Agent 会在文件变化时自动触发 rescan + sync（500ms 防抖）。

### 手动扫描、同步和日志

```bash
tsync rescan <task-id>
tsync sync <task-id>
tsync logs --limit 50
```

如果 sync task 的 target_node 是远端节点，Agent 会通过 QUIC 将文件传输到远端 Agent。

## 🛠️ 技术栈

| 模块 | 技术选型 | 原因 |
|------|----------|------|
| **语言** | Rust Stable | 性能、安全、适合系统工具 |
| **异步运行时** | Tokio | Rust 异步生态事实标准 |
| **CLI** | Clap | 成熟、易维护 |
| **本地 API** | Axum | 简洁、Tokio 原生 |
| **数据库** | SQLite | 单机 Agent 最小依赖 |
| **数据库访问** | SQLx | 迁移成熟、SQL 清晰 |
| **文件监听** | Notify | 跨平台文件监听成熟方案 |
| **哈希** | Blake3 | 适合大量文件指纹计算 |
| **传输** | Quinn | 使用成熟 QUIC 实现 |
| **TUI** | Ratatui + Crossterm | 终端仪表盘 |
| **桌面 GUI** | Slint | 原生跨平台窗口，不依赖 Electron / WebView |
| **日志** | Tracing | Rust 异步日志标准方案 |

## 📁 Workspace 结构

```text
turbo_sync/
├── crates/
│   ├── turbosync-agent/      # 本地 Agent 和控制 API
│   ├── turbosync-cli/        # tsync 命令行入口
│   ├── turbosync-core/       # 配置、模型、通用类型
│   ├── turbosync-gui/        # 桌面 GUI (Slint，独立 workspace)
│   ├── turbosync-storage/    # SQLite / SQLx 持久化
│   ├── turbosync-sync/       # 文件扫描、同步引擎、文件监听
│   ├── turbosync-transport/  # Agent-to-Agent QUIC 传输
│   └── turbosync-tui/        # 终端仪表盘 (ratatui)
├── migrations/               # SQLx migrations
├── docs/                     # 用户指南、开发自测、产品文案
└── README.md
```

## 🧪 本地开发

运行全部检查：

```bash
make check
```

等价于：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

如果运行环境限制本地端口绑定，`turbosync-transport` 的 QUIC 测试可能因为无法绑定 `127.0.0.1` 失败；在普通本机终端或允许本地端口绑定的环境中应通过。

使用临时配置和数据库路径进行手动测试：

```bash
make selftest-init
make selftest-agent
make selftest-dashboard
```

桌面 GUI 是独立 workspace，避免和 TUI 依赖发生版本冲突：

```bash
make build-gui
make gui
```

完整手动验收流程见：

```text
docs/testing.zh.md
```

推荐发布前至少完成：

- 自动检查：fmt / clippy / test 全部通过
- 单机双 Agent 远端同步：创建、更新、删除
- watch 自动同步
- TUI：状态查看、手动同步、进度显示、任务编辑、文件浏览、watch 开关

## 🚢 发布

GitHub Actions 会在 PR 和 `main` / `master` 分支 push 时自动运行格式、Clippy 和测试检查，并单独检查桌面 GUI。

发布版本通过 `v*` tag 触发，自动构建 Windows / Linux / macOS 包，包含：

- `tsync`
- `turbosync-agent`
- `tsync-gui`
- `README.md`
- `LICENSE`

发布前本地检查：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
cargo build --release --manifest-path crates/turbosync-gui/Cargo.toml
```

创建发布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

tag 版本应与根 `Cargo.toml` 的 workspace version 保持一致。当前阶段适合软推广和招募早期测试用户；更大范围推广建议等 `v0.1.0` Release 和三平台二进制包可下载后再进行。

## 📚 文档

当前文档：

```text
docs/testing.zh.md             # 开发和发布前自测流程
docs/user-guide.zh.md          # 用户使用指南（TUI 优先）
docs/product/landing.zh.md     # 产品介绍和未来官网文案
```

README 用于项目入口、当前实现状态和快速开始；`docs/user-guide.zh.md` 面向日常使用；`docs/testing.zh.md` 面向开发和发布前验收。
