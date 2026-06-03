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

第一阶段暂不实现：

- Web UI
- 用户系统
- 权限系统
- 集群
- 云盘能力
- 在线预览
- 文档协作

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

下一步 MVP 工作：

- ⏳ 文件扫描和索引
- ⏳ 手动同步
- ⏳ 文件监听
- ⏳ Agent-to-Agent 文件传输

## 🏗️ 架构设计

```text
CLI
 ↓ localhost HTTP
TurboSync Agent
 ├─ Local Control API
 ├─ SQLite State DB
 ├─ Watcher
 ├─ Scanner / Indexer
 ├─ Sync Engine
 └─ Transport
```

第一阶段采用 **source → target** 的单向同步模型，源目录是权威数据源。重命名可以先按“删除旧路径 + 新增新路径”处理，后续再扩展冲突处理和双向同步。

## 🚀 快速开始

### 1. 初始化本地状态

```bash
tsync init
```

这会创建：

- 配置文件
- SQLite 数据库
- 初始节点 ID

### 2. 启动 Agent

```bash
tsync agent run
```

Agent 会在本地启动控制 API，默认监听：

```text
127.0.0.1:38745
```

### 3. 查看状态

```bash
tsync status
```

### 4. 管理节点

```bash
tsync node add nas 192.168.1.20:38746
tsync node list
tsync node remove <node-id>
```

### 5. 管理同步任务

```bash
tsync task add documents \
  --source ~/Documents \
  --target-node <node-id> \
  --target-path /backup/Documents

tsync task list
tsync task remove <task-id>
```

### 6. 后续同步命令

这些命令属于后续 MVP 里程碑：

```bash
tsync sync <task-id>
tsync rescan <task-id>
tsync logs --limit 50
```

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
| **日志** | Tracing | Rust 异步日志标准方案 |

## 📁 Workspace 结构

```text
turbo_sync/
├── crates/
│   ├── turbosync-agent/      # 本地 Agent 和控制 API
│   ├── turbosync-cli/        # tsync 命令行入口
│   ├── turbosync-core/       # 配置、模型、通用类型
│   ├── turbosync-storage/    # SQLite / SQLx 持久化
│   ├── turbosync-sync/       # 文件扫描和同步引擎
│   └── turbosync-transport/  # Agent-to-Agent 传输
├── migrations/               # SQLx migrations
├── docs/product/             # 产品介绍和未来官网文案
└── README.md
```

## 🧪 本地开发

运行全部检查：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

使用临时配置和数据库路径进行手动测试：

```bash
export TURBOSYNC_CONFIG=/tmp/turbosync/config.toml
export TURBOSYNC_DB=/tmp/turbosync/turbosync.db

tsync init
tsync agent run
```

## 📚 文档

产品介绍文案保存在：

```text
docs/product/landing.zh.md
```

README 用于开发者入口、当前实现状态和快速开始；`docs/product/landing.zh.md` 用作未来官网首页或项目介绍页的源文案。
