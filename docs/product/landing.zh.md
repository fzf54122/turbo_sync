# TurboSync

## 简单、可控、面向个人设备的文件同步工具

TurboSync 是一个使用 Rust 编写的跨平台文件同步工具。

它的目标不是变成另一个复杂的云盘系统，也不是让用户理解一堆同步概念。第一阶段只解决一个明确问题：把一个目录稳定同步到另一台设备。

```bash
tsync init
tsync agent run
tsync node add nas 192.168.1.20:38746
tsync task add documents --source ~/Documents --target-node nas --target-path /backup/Documents
tsync sync documents
```

## 为什么做 TurboSync？

现有工具各有优势，但也有明显门槛：

- `rsync` 很强，但参数和使用模型对普通用户不够友好
- Syncthing 功能完整，但第一次配置需要理解较多概念
- 网盘类产品方便，但不适合所有本地、NAS、Homelab 场景
- 自己写脚本可以解决一时问题，但长期维护和错误恢复都麻烦

TurboSync 关注一个更小的问题：

> 让个人开发者、NAS 用户、Linux 用户和 Homelab 用户，用更少配置完成稳定的文件同步。

## 第一阶段 MVP 做什么？

TurboSync 第一阶段只做文件同步最核心的闭环。

包括：

- CLI 命令行工具
- 本地 Agent
- 节点注册
- 同步任务配置
- 文件夹扫描
- 文件监听
- 增量同步
- 删除同步
- 断线重连
- 同步日志
- SQLite 本地状态存储

不做：

- Web UI
- 用户系统
- 权限系统
- 集群
- 云盘
- 在线预览
- 文档协作
- 企业管理后台

这让第一版可以更快跑起来，也更容易长期维护。

## 适合谁？

### 开发者

把代码、配置、笔记、工作目录同步到另一台机器或 NAS。

### NAS 用户

把本机目录同步到 NAS，不需要部署复杂平台。

### Linux 用户

用 CLI 管理同步任务，适合服务器、工作站和本地脚本集成。

### Homelab 用户

在几台自有设备之间建立简单、透明、可观测的同步链路。

## 设计原则

### 优先可运行

第一版先完成：

```text
本机目录 → 远程节点目录
```

不一开始做复杂双向同步和冲突合并。

### 优先成熟生态

底层能力尽量使用成熟 Rust crate：

- Tokio：异步运行时
- Clap：CLI
- Axum：本地控制 API
- SQLite + SQLx：本地状态
- Notify：文件监听
- Blake3：文件指纹
- Quinn：QUIC 传输
- Ratatui + Crossterm：终端仪表盘
- Tracing：日志

不自研 QUIC、不自研数据库、不自研文件监听。

### 优先少概念

用户不需要理解集群、空间、权限、团队、Bucket、Drive。

第一阶段只需要理解：

```text
Node
Task
Source
Target
Sync
```

### 优先单人可维护

TurboSync 不是企业级平台起步。

它先是一个个人开发者也能维护、调试、发布的工具。

## 工作方式

TurboSync 使用本地 Agent 运行同步任务。

```text
CLI / TUI Dashboard
 ↓
Local Agent
 ↓
File Watcher (notify, 500ms debounce)
 ↓
Sync Engine (本地 / 远端路由)
 ↓
Remote Agent (QUIC)
```

当源目录发生变化：

1. Agent 监听到文件事件
2. 对路径进行防抖和过滤
3. 扫描文件状态
4. 计算文件 hash
5. 生成同步操作
6. 传输到目标节点
7. 接收端原子写入文件
8. 写入同步日志

文件监听事件只作为提示，最终状态以扫描和 SQLite 索引为准。

## 当前状态

已经完成：

- Rust Cargo Workspace
- CLI / Agent 基础骨架
- `tsync init`
- 平台标准配置目录
- TOML 配置文件
- SQLite 数据库初始化
- SQLx migrations
- Agent 本地控制 API
- CLI 连接本地 Agent 管理节点和任务
- 文件扫描和 SQLite 索引
- 本地和远端同步（QUIC 传输）
- 文件监听 + 500ms 防抖
- CLI watch 子命令
- 终端仪表盘 (`tsync dashboard`)
- 同步日志

第一阶段 MVP 闭环已完成。下一阶段：GUI 客户端。

## 一句话介绍

TurboSync 是一个面向开发者、NAS 和 Homelab 用户的 Rust 文件同步工具，用 CLI 和本地 Agent 提供简单、可控、可自托管的目录同步体验。
