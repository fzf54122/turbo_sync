import { useState, useEffect } from 'react'
import { marked } from 'marked'
import hljs from 'highlight.js/lib/core'
import bash from 'highlight.js/lib/languages/bash'
import rust from 'highlight.js/lib/languages/rust'
import 'highlight.js/styles/github-dark.css'
import './App.css'

hljs.registerLanguage('bash', bash)
hljs.registerLanguage('rust', rust)

const REPO_URL = 'https://github.com/fzf54122/turbo_sync'

function getSystemTheme() {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function applyTheme(newTheme) {
  const resolvedTheme = newTheme === 'auto' ? getSystemTheme() : newTheme
  document.documentElement.setAttribute('data-theme', resolvedTheme)
}

function App() {
  const [theme, setTheme] = useState('auto')
  const [lang, setLang] = useState('zh')
  const [page, setPage] = useState('home')
  const [activeDoc, setActiveDoc] = useState('intro')
  const [searchQuery, setSearchQuery] = useState('')
  const [showThemeMenu, setShowThemeMenu] = useState(false)
  const [showLangMenu, setShowLangMenu] = useState(false)

  useEffect(() => {
    const savedTheme = localStorage.getItem('theme') || 'auto'
    const savedLang = localStorage.getItem('lang') || 'zh'
    setTheme(savedTheme)
    setLang(savedLang)
    applyTheme(savedTheme)

    const syncPageFromHash = () => {
      const hashPage = window.location.hash.replace('#/', '')
      setPage(['docs', 'download'].includes(hashPage) ? hashPage : 'home')
    }

    syncPageFromHash()
    window.addEventListener('hashchange', syncPageFromHash)

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleSystemThemeChange = () => {
      if ((localStorage.getItem('theme') || 'auto') === 'auto') {
        applyTheme('auto')
      }
    }

    mediaQuery.addEventListener('change', handleSystemThemeChange)
    return () => {
      window.removeEventListener('hashchange', syncPageFromHash)
      mediaQuery.removeEventListener('change', handleSystemThemeChange)
    }
  }, [])

  useEffect(() => {
    const titles = {
      zh: {
        home: '飞梭同步 - 跨平台文件同步工具',
        docs: '文档 - 飞梭同步',
        download: '下载 - 飞梭同步'
      },
      en: {
        home: 'TurboSync - Cross-platform File Sync',
        docs: 'Docs - TurboSync',
        download: 'Download - TurboSync'
      }
    }
    document.title = titles[lang][page]
  }, [page, lang])

  useEffect(() => {
    if (page === 'docs') {
      document.querySelectorAll('pre code').forEach((block) => {
        hljs.highlightElement(block)
      })
    }
  }, [page, activeDoc, lang])

  const navigateTo = (nextPage) => {
    setPage(nextPage)
    window.location.hash = nextPage === 'home' ? '' : `/${nextPage}`
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }

  const handleThemeChange = (newTheme) => {
    setTheme(newTheme)
    localStorage.setItem('theme', newTheme)
    applyTheme(newTheme)
    setShowThemeMenu(false)
  }

  const handleLangChange = (newLang) => {
    setLang(newLang)
    localStorage.setItem('lang', newLang)
    setShowLangMenu(false)
  }

  const t = {
    zh: {
      logo: '飞梭同步',
      features: '功能特性',
      docs: '文档',
      download: '下载',
      github: 'GitHub',
      heroTitle: '飞梭同步',
      heroSubtitle: 'Intelligence, Crafted.',
      heroDesc: '跨平台文件同步工具，适合开发目录、NAS、服务器和 Homelab 场景',
      productTitle: 'TurboSync Agent',
      productDesc: '本机后台同步服务，负责扫描、监听、同步和传输',
      downloadBtn: '立即下载',
      ctaTitle: '探索智能，创造可能',
      ctaDesc: 'Explore intelligence. Create possibility.',
      ctaBtn: '开始使用',
      docsTitle: '飞梭同步文档',
      docsDesc: '在官网了解核心概念、使用流程和常用命令。',
      downloadTitle: '下载飞梭同步',
      downloadDesc: '选择适合日常桌面、服务器或开发场景的入口。',
      sourceCode: '源码仓库',
      releaseDetails: 'Release 详情',
      backHome: '返回首页',
      footer: '© 2024 飞梭同步 · MIT License',
      theme: {
        light: '浅色模式',
        dark: '深色模式',
        auto: '自动模式'
      }
    },
    en: {
      logo: 'TurboSync',
      features: 'Features',
      docs: 'Docs',
      download: 'Download',
      github: 'GitHub',
      heroTitle: 'TurboSync',
      heroSubtitle: 'Intelligence, Crafted.',
      heroDesc: 'Cross-platform file sync tool for development, NAS, servers and Homelab',
      productTitle: 'TurboSync Agent',
      productDesc: 'Local background sync service for scanning, monitoring, syncing and transferring',
      downloadBtn: 'Download Now',
      ctaTitle: 'Explore Intelligence, Create Possibility',
      ctaDesc: 'Explore intelligence. Create possibility.',
      ctaBtn: 'Get Started',
      docsTitle: 'TurboSync Docs',
      docsDesc: 'Learn the concepts, workflow, and common commands on the official site.',
      downloadTitle: 'Download TurboSync',
      downloadDesc: 'Choose the right entry for desktop, server, or developer workflows.',
      sourceCode: 'Source Code',
      releaseDetails: 'Release Details',
      backHome: 'Back Home',
      footer: '© 2024 TurboSync · MIT License',
      theme: {
        light: 'Light',
        dark: 'Dark',
        auto: 'Auto'
      }
    }
  }

  const content = t[lang]

  const docNav = lang === 'zh' ? [
    { id: 'intro', title: '快速开始', icon: '📖' },
    { id: 'features', title: '功能特性', icon: '✨' },
    { id: 'arch', title: '技术架构', icon: '🏗️' },
    { id: 'cli', title: 'CLI 命令', icon: '⌨️' },
  ] : [
    { id: 'intro', title: 'Quick Start', icon: '📖' },
    { id: 'features', title: 'Features', icon: '✨' },
    { id: 'arch', title: 'Architecture', icon: '🏗️' },
    { id: 'cli', title: 'CLI Commands', icon: '⌨️' },
  ]

  const docContent = {
    zh: {
      intro: {
        title: '快速开始',
        content: `欢迎使用飞梭同步！这是一款基于 Rust 的跨平台文件同步工具，专为开发者、NAS 用户和 Homelab 爱好者设计。

## 为什么选择飞梭同步？

与传统同步工具不同，飞梭同步提供：

- **实时监听**：基于文件系统事件，毫秒级响应文件变化
- **增量传输**：只同步变化的部分，节省带宽和时间
- **双向同步**：多节点互相同步，灵活配置同步方向
- **智能冲突处理**：自动检测并标记冲突文件
- **轻量高效**：Rust 实现，低内存占用，高性能传输
- **跨平台**：支持 Linux、macOS、Windows

## 安装方式

### 方式一：桌面端（推荐新手）

访问 [GitHub Releases](${REPO_URL}/releases) 下载适合你系统的安装包：

- **Windows**：\`.msi\` 安装包
- **macOS**：\`.dmg\` 镜像文件
- **Linux**：\`.AppImage\` 或 \`.deb\` 包

### 方式二：命令行工具

适合服务器和自动化场景：

\`\`\`bash
# Linux / macOS
curl -fsSL ${REPO_URL}/install.sh | bash

# 或手动下载
wget ${REPO_URL}/releases/latest/download/tsync-linux-x64.tar.gz
tar -xzf tsync-linux-x64.tar.gz
sudo mv tsync /usr/local/bin/
\`\`\`

### 方式三：从源码构建

适合开发者和高级用户：

\`\`\`bash
# 克隆仓库
git clone ${REPO_URL}
cd turbo_sync

# 构建所有组件
cargo build --release

# 或只构建特定组件
cargo build --release -p turbosync-agent
cargo build --release -p turbosync-cli
\`\`\`

## 第一次使用

### 1. 初始化配置

首次运行时，需要初始化配置文件：

\`\`\`bash
tsync init
\`\`\`

这会在 \`~/.config/turbosync/\` 创建配置目录和数据库。

### 2. 启动后台服务

所有同步操作都依赖 Agent 服务：

\`\`\`bash
# 前台运行（查看日志）
tsync agent run

# 或后台运行
tsync agent start
\`\`\`

### 3. 添加第一个节点

节点代表一台设备或服务器：

\`\`\`bash
# 添加本机节点
tsync node add localhost 127.0.0.1:9527

# 添加远程节点
tsync node add nas 192.168.1.100:9527
\`\`\`

### 4. 创建同步任务

定义源目录、目标节点和目标路径：

\`\`\`bash
# 同步本地目录到远程
tsync task create ~/Documents nas:/backup/docs

# 查看任务列表
tsync task list
\`\`\`

### 5. 开始同步

\`\`\`bash
# 手动触发同步
tsync sync

# 或启用自动同步
tsync task enable <task-id>
\`\`\`

## 核心概念

### 节点（Node）

节点是同步网络中的一台设备，可以是：
- 本机（localhost）
- 局域网内的 NAS 或服务器
- 云服务器（需配置防火墙）

每个节点运行一个 Agent 服务，监听指定端口（默认 9527）。

### 任务（Task）

同步任务包含三个要素：
1. **源目录**：要同步的本地目录
2. **目标节点**：接收文件的节点
3. **目标路径**：目标节点上的存储路径

### Agent

后台同步服务，职责包括：
- 扫描目录结构并记录文件元数据
- 监听文件系统变化（创建、修改、删除）
- 生成同步计划并执行传输
- 处理冲突和错误重试

### 同步模式

- **单向同步**：源 → 目标（最稳定）
- **双向同步**：源 ⇄ 目标（实验性功能）
- **镜像模式**：目标完全镜像源（会删除目标多余文件）

## 下一步

- 查看 [功能特性](#) 了解更多高级功能
- 阅读 [CLI 命令](#) 掌握命令行用法
- 参考 [技术架构](#) 理解实现原理`
      },
      features: {
        title: '功能特性',
        content: `飞梭同步为现代文件同步场景精心打造，每个功能都经过深思熟虑。

## 实时监听与增量同步

### 文件系统监听

基于 \`notify\` 库的跨平台文件系统事件监听：

\`\`\`rust
use notify::{Watcher, RecursiveMode, Event};

let mut watcher = notify::recommended_watcher(|res: Result<Event, _>| {
    match res {
        Ok(event) => handle_fs_event(event),
        Err(e) => log::error!("Watch error: {:?}", e),
    }
})?;

watcher.watch(path, RecursiveMode::Recursive)?;
\`\`\`

**触发时机**：
- 文件创建（\`CREATE\`）
- 文件修改（\`MODIFY\`）
- 文件删除（\`REMOVE\`）
- 文件移动（\`RENAME\`）

### 增量传输算法

只传输变化的部分，大幅节省带宽：

\`\`\`rust
fn calculate_delta(old_hash: &str, new_hash: &str) -> Vec<Block> {
    // 使用 rolling hash 算法（类似 rsync）
    // 只传输差异块
    let blocks = compare_blocks(old_hash, new_hash);
    blocks.into_iter()
        .filter(|b| b.changed)
        .collect()
}
\`\`\`

**性能对比**：
- 全量传输：1GB 文件需要传输 1GB
- 增量传输：修改 10MB 只需传输 10MB
- 传输效率提升：90%+

## 双向同步与冲突处理

### 同步模式

#### 1. 单向同步（推荐）

源目录的变化单向同步到目标：

\`\`\`bash
# 本地 → 远程
tsync task create ~/work nas:/backup/work --mode one-way

# 远程 → 本地
tsync task create nas:/data ~/local-data --mode one-way
\`\`\`

#### 2. 双向同步

两个目录互相同步变化：

\`\`\`bash
tsync task create ~/docs nas:/docs --mode bidirectional
\`\`\`

#### 3. 镜像模式

目标完全镜像源，删除目标多余文件：

\`\`\`bash
tsync task create ~/deploy server:/www --mode mirror
\`\`\`

### 冲突检测

当双向同步中，两端同时修改同一文件时：

\`\`\`bash
# 自动生成冲突文件
document.txt
document.txt.conflict-2024-06-11-14-30-localhost
document.txt.conflict-2024-06-11-14-31-nas
\`\`\`

**冲突策略**：
- \`keep-both\`：保留所有版本（默认）
- \`keep-newer\`：保留最新修改的版本
- \`keep-local\`：本地优先
- \`keep-remote\`：远程优先

## 过滤规则与忽略模式

### .tsyncignore 文件

类似 \`.gitignore\` 的语法：

\`\`\`bash
# 忽略日志文件
*.log
*.tmp

# 忽略依赖目录
node_modules/
target/
__pycache__/

# 忽略系统文件
.DS_Store
Thumbs.db
desktop.ini

# 忽略敏感文件
.env
*.key
*.pem
\`\`\`

### 全局过滤规则

在配置文件中设置全局规则：

\`\`\`bash
tsync config set ignore.patterns '*.log,node_modules/,target/'
\`\`\`

### 大文件处理

\`\`\`bash
# 设置单文件大小限制（默认 1GB）
tsync config set max_file_size 2GB

# 跳过大于 100MB 的文件
tsync task create ~/videos nas:/media --max-file-size 100MB
\`\`\`

## 传输协议与安全

### QUIC 协议

远程传输使用 QUIC 协议（基于 UDP）：

**优势**：
- 比 TCP 更快的连接建立
- 0-RTT 恢复连接
- 内置多路复用，避免队头阻塞
- 自动拥塞控制

\`\`\`rust
// 使用 quinn 库实现 QUIC
let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
let connection = endpoint.connect(addr, "turbosync")?.await?;
\`\`\`

### 加密传输

\`\`\`bash
# 启用 TLS 加密
tsync node add nas 192.168.1.100:9527 --tls

# 使用自签名证书
tsync node add nas 192.168.1.100:9527 --tls --insecure
\`\`\`

## 性能优化

### 并发传输

\`\`\`bash
# 设置并发连接数（默认 4）
tsync config set transfer.concurrency 8

# 设置单文件块大小（默认 4MB）
tsync config set transfer.chunk_size 8MB
\`\`\`

### 压缩传输

\`\`\`bash
# 启用传输压缩（适合文本文件）
tsync task create ~/code nas:/backup --compress

# 跳过已压缩文件的压缩
tsync config set compress.skip_extensions 'zip,gz,jpg,png,mp4'
\`\`\`

## 监控与日志

### 实时状态

\`\`\`bash
# 查看所有任务状态
tsync status

# 查看特定任务详情
tsync task info <task-id>

# 实时监控传输速度
tsync monitor
\`\`\`

### 日志级别

\`\`\`bash
# 设置日志级别
tsync config set log.level debug

# 查看日志
tsync log

# 导出日志
tsync log --export turbosync.log
\`\`\``
      },
      arch: {
        title: '技术架构',
        content: `## 整体架构

\`\`\`
┌─────────────────────────────────────┐
│         GUI / TUI / CLI             │
│  (用户界面层)                        │
└──────────────┬──────────────────────┘
               │ IPC / HTTP API
┌──────────────▼──────────────────────┐
│         TurboSync Agent             │
│  - 文件扫描                          │
│  - 变化监听                          │
│  - 同步调度                          │
│  - 文件传输                          │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│         SQLite Database             │
│  - 节点信息                          │
│  - 任务配置                          │
│  - 同步日志                          │
└─────────────────────────────────────┘
\`\`\`

## 核心模块

### 文件扫描器

使用 Rust 的 \`walkdir\` 进行高效目录扫描：

\`\`\`rust
use walkdir::WalkDir;

pub async fn scan_directory(path: &Path) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let hash = calculate_hash(entry.path()).await?;
            files.push(FileInfo {
                path: entry.path().to_path_buf(),
                hash,
                size: entry.metadata()?.len(),
            });
        }
    }
    Ok(files)
}
\`\`\`

### 监听器

基于 \`notify\` 库实现跨平台文件系统监听：

\`\`\`rust
use notify::{Watcher, RecursiveMode, Event};

let (tx, rx) = channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(path, RecursiveMode::Recursive)?;

for event in rx {
    match event {
        Ok(Event { kind: EventKind::Create(_), paths, .. }) => {
            handle_create(paths).await?;
        }
        Ok(Event { kind: EventKind::Modify(_), paths, .. }) => {
            handle_modify(paths).await?;
        }
        _ => {}
    }
}
\`\`\`

### 传输层

本地使用文件复制，远程使用 QUIC 协议：

\`\`\`rust
// 本地传输
tokio::fs::copy(src, dest).await?;

// QUIC 传输
let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
let conn = endpoint.connect(addr, "turbosync")?.await?;
let mut stream = conn.open_uni().await?;
tokio::io::copy(&mut file, &mut stream).await?;
\`\`\`

## 性能优化

- **并发扫描**：使用 tokio 并发处理多个目录
- **增量同步**：只传输变化的文件块
- **批量操作**：数据库批量插入减少 I/O
- **内存映射**：大文件使用 mmap 读取`
      },
      cli: {
        title: 'CLI 命令',
        content: `完整的命令行工具，支持所有同步操作。

## 基础命令

### 初始化配置

首次运行时需要初始化：

\`\`\`bash
# 初始化配置文件
tsync init

# 查看配置路径
tsync config path

# 编辑配置文件
tsync config edit
\`\`\`

### Agent 管理

\`\`\`bash
# 前台运行（查看日志）
tsync agent run

# 后台启动
tsync agent start

# 停止服务
tsync agent stop

# 重启服务
tsync agent restart

# 查看状态
tsync agent status
\`\`\`

## 节点管理

### 添加节点

\`\`\`bash
# 添加本地节点
tsync node add localhost 127.0.0.1:9527

# 添加远程节点
tsync node add nas 192.168.1.100:9527

# 添加节点并启用 TLS
tsync node add server example.com:9527 --tls

# 添加节点并设置别名
tsync node add prod-server 10.0.0.1:9527 --alias production
\`\`\`

### 列出节点

\`\`\`bash
# 列出所有节点
tsync node list

# 列出在线节点
tsync node list --online

# 显示详细信息
tsync node list --verbose
\`\`\`

### 节点操作

\`\`\`bash
# 测试节点连接
tsync node ping nas

# 查看节点详情
tsync node info nas

# 更新节点地址
tsync node update nas --address 192.168.1.101:9527

# 删除节点
tsync node remove nas
\`\`\`

## 任务管理

### 创建任务

\`\`\`bash
# 基础语法
tsync task create <source> <target-node>:<target-path>

# 单向同步
tsync task create ~/Documents nas:/backup/docs

# 双向同步
tsync task create ~/code nas:/code --mode bidirectional

# 镜像模式（删除目标多余文件）
tsync task create ~/deploy server:/www --mode mirror

# 设置过滤规则
tsync task create ~/project nas:/project --ignore "*.log,node_modules/"

# 启用压缩
tsync task create ~/text nas:/text --compress
\`\`\`

### 任务列表

\`\`\`bash
# 列出所有任务
tsync task list

# 列出活动任务
tsync task list --active

# 列出暂停任务
tsync task list --paused

# JSON 格式输出
tsync task list --json
\`\`\`

### 任务操作

\`\`\`bash
# 查看任务详情
tsync task info <task-id>

# 启用任务
tsync task enable <task-id>

# 暂停任务
tsync task pause <task-id>

# 恢复任务
tsync task resume <task-id>

# 删除任务
tsync task remove <task-id>

# 强制删除（包括历史数据）
tsync task remove <task-id> --force
\`\`\`

## 同步操作

### 手动同步

\`\`\`bash
# 同步所有任务
tsync sync

# 同步指定任务
tsync sync <task-id>

# 强制全量同步（忽略缓存）
tsync sync <task-id> --full

# 仅执行一次
tsync sync <task-id> --once
\`\`\`

### 监控状态

\`\`\`bash
# 查看所有任务状态
tsync status

# 实时监控
tsync monitor

# 查看传输速度
tsync monitor --speed

# 导出状态报告
tsync status --export report.json
\`\`\`

## 日志查看

\`\`\`bash
# 查看最近日志
tsync log

# 查看指定任务日志
tsync log --task <task-id>

# 查看错误日志
tsync log --level error

# 实时跟踪日志
tsync log --follow

# 导出日志
tsync log --export turbosync.log

# 清理旧日志
tsync log clean --older-than 7d
\`\`\`

## 配置管理

\`\`\`bash
# 查看配置
tsync config get

# 设置配置项
tsync config set <key> <value>

# 常用配置
tsync config set log.level debug
tsync config set transfer.concurrency 8
tsync config set transfer.chunk_size 8MB
tsync config set compress.enabled true

# 重置配置
tsync config reset
\`\`\`

## 高级功能

### 导入导出

\`\`\`bash
# 导出配置
tsync export --output backup.json

# 导入配置
tsync import --input backup.json

# 导出特定任务
tsync task export <task-id> --output task.json

# 导入任务
tsync task import --input task.json
\`\`\`

### 诊断工具

\`\`\`bash
# 运行诊断
tsync diagnose

# 检查文件系统权限
tsync diagnose permissions

# 检查网络连接
tsync diagnose network

# 生成诊断报告
tsync diagnose --report
\`\`\`

### 数据库维护

\`\`\`bash
# 数据库统计
tsync db stats

# 清理缓存
tsync db clean

# 优化数据库
tsync db optimize

# 备份数据库
tsync db backup --output backup.db
\`\`\`

## 脚本示例

### 自动备份脚本

\`\`\`bash
#!/bin/bash
# backup.sh - 定时备份脚本

# 确保 Agent 运行
tsync agent status || tsync agent start

# 执行所有备份任务
tsync sync --tag backup

# 检查是否有错误
if [ $? -eq 0 ]; then
    echo "Backup completed successfully"
else
    echo "Backup failed" >&2
    exit 1
fi
\`\`\`

### 批量创建任务

\`\`\`bash
#!/bin/bash
# setup-sync.sh - 批量创建同步任务

declare -A tasks=(
    ["~/Documents"]="nas:/backup/docs"
    ["~/Pictures"]="nas:/backup/pics"
    ["~/Videos"]="nas:/backup/videos"
)

for source in "\${!tasks[@]}"; do
    target="\${tasks[$source]}"
    echo "Creating task: $source -> $target"
    tsync task create "$source" "$target" --mode one-way
done

echo "All tasks created. Starting sync..."
tsync sync
\`\`\``
      },
    },
    en: {
      intro: {
        title: 'Quick Start',
        content: `Welcome to TurboSync! A cross-platform file synchronization tool built with Rust, designed for developers, NAS users, and Homelab enthusiasts.

## Why TurboSync?

Unlike traditional sync tools, TurboSync offers:

- **Real-time Monitoring**: Filesystem event-based, millisecond-level response to file changes
- **Incremental Transfer**: Syncs only changed portions, saving bandwidth and time
- **Bidirectional Sync**: Multi-node mutual sync with flexible direction configuration
- **Smart Conflict Resolution**: Automatic conflict detection and flagging
- **Lightweight & Efficient**: Rust implementation with low memory footprint and high-performance transfer
- **Cross-platform**: Supports Linux, macOS, Windows

## Installation Methods

### Method 1: Desktop App (Recommended for Beginners)

Visit [GitHub Releases](${REPO_URL}/releases) to download the installer for your system:

- **Windows**: \`.msi\` installer
- **macOS**: \`.dmg\` disk image
- **Linux**: \`.AppImage\` or \`.deb\` package

### Method 2: Command Line Tool

Suitable for servers and automation scenarios:

\`\`\`bash
# Linux / macOS
curl -fsSL ${REPO_URL}/install.sh | bash

# Or manual download
wget ${REPO_URL}/releases/latest/download/tsync-linux-x64.tar.gz
tar -xzf tsync-linux-x64.tar.gz
sudo mv tsync /usr/local/bin/
\`\`\`

### Method 3: Build from Source

For developers and advanced users:

\`\`\`bash
# Clone repository
git clone ${REPO_URL}
cd turbo_sync

# Build all components
cargo build --release

# Or build specific components
cargo build --release -p turbosync-agent
cargo build --release -p turbosync-cli
\`\`\`

## First Use

### 1. Initialize Configuration

Initialize the config file on first run:

\`\`\`bash
tsync init
\`\`\`

This creates the configuration directory and database at \`~/.config/turbosync/\`.

### 2. Start Background Service

All sync operations depend on the Agent service:

\`\`\`bash
# Run in foreground (view logs)
tsync agent run

# Or run in background
tsync agent start
\`\`\`

### 3. Add First Node

A node represents a device or server:

\`\`\`bash
# Add local node
tsync node add localhost 127.0.0.1:9527

# Add remote node
tsync node add nas 192.168.1.100:9527
\`\`\`

### 4. Create Sync Task

Define source directory, target node, and target path:

\`\`\`bash
# Sync local directory to remote
tsync task create ~/Documents nas:/backup/docs

# View task list
tsync task list
\`\`\`

### 5. Start Syncing

\`\`\`bash
# Manually trigger sync
tsync sync

# Or enable auto sync
tsync task enable <task-id>
\`\`\`

## Core Concepts

### Node

A node is a device in the sync network, which can be:
- Local machine (localhost)
- NAS or server on LAN
- Cloud server (firewall configuration required)

Each node runs an Agent service listening on a specified port (default 9527).

### Task

A sync task contains three elements:
1. **Source Directory**: Local directory to sync
2. **Target Node**: Node receiving files
3. **Target Path**: Storage path on target node

### Agent

Background sync service with responsibilities including:
- Scanning directory structure and recording file metadata
- Monitoring filesystem changes (create, modify, delete)
- Generating sync plans and executing transfers
- Handling conflicts and error retries

### Sync Modes

- **One-way Sync**: Source → Target (most stable)
- **Bidirectional Sync**: Source ⇄ Target (experimental)
- **Mirror Mode**: Target completely mirrors source (deletes extra files on target)

## Next Steps

- Check [Features](#) to learn about advanced capabilities
- Read [CLI Commands](#) to master command-line usage
- Refer to [Architecture](#) to understand implementation principles`
      },
      features: {
        title: 'Features',
        content: `TurboSync is carefully crafted for modern file sync scenarios, with every feature thoughtfully designed.

## Real-time Monitoring & Incremental Sync

### Filesystem Monitoring

Cross-platform filesystem event monitoring based on the \`notify\` library:

\`\`\`rust
use notify::{Watcher, RecursiveMode, Event};

let mut watcher = notify::recommended_watcher(|res: Result<Event, _>| {
    match res {
        Ok(event) => handle_fs_event(event),
        Err(e) => log::error!("Watch error: {:?}", e),
    }
})?;

watcher.watch(path, RecursiveMode::Recursive)?;
\`\`\`

**Trigger Events**:
- File create (\`CREATE\`)
- File modify (\`MODIFY\`)
- File delete (\`REMOVE\`)
- File move (\`RENAME\`)

### Incremental Transfer Algorithm

Transfers only changed portions, dramatically saving bandwidth:

\`\`\`rust
fn calculate_delta(old_hash: &str, new_hash: &str) -> Vec<Block> {
    // Using rolling hash algorithm (similar to rsync)
    // Only transfer diff blocks
    let blocks = compare_blocks(old_hash, new_hash);
    blocks.into_iter()
        .filter(|b| b.changed)
        .collect()
}
\`\`\`

**Performance Comparison**:
- Full transfer: 1GB file requires 1GB transfer
- Incremental transfer: 10MB modification requires only 10MB transfer
- Transfer efficiency improvement: 90%+

## Bidirectional Sync & Conflict Resolution

### Sync Modes

#### 1. One-way Sync (Recommended)

Source changes sync to target in one direction:

\`\`\`bash
# Local → Remote
tsync task create ~/work nas:/backup/work --mode one-way

# Remote → Local
tsync task create nas:/data ~/local-data --mode one-way
\`\`\`

#### 2. Bidirectional Sync

Two directories sync changes mutually:

\`\`\`bash
tsync task create ~/docs nas:/docs --mode bidirectional
\`\`\`

#### 3. Mirror Mode

Target completely mirrors source, deleting extra files on target:

\`\`\`bash
tsync task create ~/deploy server:/www --mode mirror
\`\`\`

### Conflict Detection

When both ends modify the same file in bidirectional sync:

\`\`\`bash
# Automatically generates conflict files
document.txt
document.txt.conflict-2024-06-11-14-30-localhost
document.txt.conflict-2024-06-11-14-31-nas
\`\`\`

**Conflict Strategies**:
- \`keep-both\`: Keep all versions (default)
- \`keep-newer\`: Keep most recently modified
- \`keep-local\`: Local takes precedence
- \`keep-remote\`: Remote takes precedence

## Filter Rules & Ignore Patterns

### .tsyncignore File

Syntax similar to \`.gitignore\`:

\`\`\`bash
# Ignore log files
*.log
*.tmp

# Ignore dependency directories
node_modules/
target/
__pycache__/

# Ignore system files
.DS_Store
Thumbs.db
desktop.ini

# Ignore sensitive files
.env
*.key
*.pem
\`\`\`

### Global Filter Rules

Set global rules in config file:

\`\`\`bash
tsync config set ignore.patterns '*.log,node_modules/,target/'
\`\`\`

### Large File Handling

\`\`\`bash
# Set single file size limit (default 1GB)
tsync config set max_file_size 2GB

# Skip files larger than 100MB
tsync task create ~/videos nas:/media --max-file-size 100MB
\`\`\`

## Transfer Protocol & Security

### QUIC Protocol

Remote transfers use QUIC protocol (UDP-based):

**Advantages**:
- Faster connection establishment than TCP
- 0-RTT connection resumption
- Built-in multiplexing, avoiding head-of-line blocking
- Automatic congestion control

\`\`\`rust
// Using quinn library for QUIC
let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
let connection = endpoint.connect(addr, "turbosync")?.await?;
\`\`\`

### Encrypted Transfer

\`\`\`bash
# Enable TLS encryption
tsync node add nas 192.168.1.100:9527 --tls

# Use self-signed certificate
tsync node add nas 192.168.1.100:9527 --tls --insecure
\`\`\`

## Performance Optimization

### Concurrent Transfer

\`\`\`bash
# Set concurrent connections (default 4)
tsync config set transfer.concurrency 8

# Set chunk size per file (default 4MB)
tsync config set transfer.chunk_size 8MB
\`\`\`

### Compressed Transfer

\`\`\`bash
# Enable transfer compression (suitable for text files)
tsync task create ~/code nas:/backup --compress

# Skip compression for already compressed files
tsync config set compress.skip_extensions 'zip,gz,jpg,png,mp4'
\`\`\`

## Monitoring & Logs

### Real-time Status

\`\`\`bash
# View all task statuses
tsync status

# View specific task details
tsync task info <task-id>

# Monitor transfer speed in real-time
tsync monitor
\`\`\`

### Log Levels

\`\`\`bash
# Set log level
tsync config set log.level debug

# View logs
tsync log

# Export logs
tsync log --export turbosync.log
\`\`\``
      },
      arch: {
        title: 'Architecture',
        content: `## Overall Architecture

\`\`\`
┌─────────────────────────────────────┐
│         GUI / TUI / CLI             │
│  (User Interface Layer)             │
└──────────────┬──────────────────────┘
               │ IPC / HTTP API
┌──────────────▼──────────────────────┐
│         TurboSync Agent             │
│  - File Scanning                    │
│  - Change Monitoring                │
│  - Sync Scheduling                  │
│  - File Transfer                    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│         SQLite Database             │
│  - Node Info                        │
│  - Task Config                      │
│  - Sync Logs                        │
└─────────────────────────────────────┘
\`\`\`

## Core Modules

### File Scanner

Efficient directory scanning using Rust's \`walkdir\`:

\`\`\`rust
use walkdir::WalkDir;

pub async fn scan_directory(path: &Path) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let hash = calculate_hash(entry.path()).await?;
            files.push(FileInfo {
                path: entry.path().to_path_buf(),
                hash,
                size: entry.metadata()?.len(),
            });
        }
    }
    Ok(files)
}
\`\`\`

### Watcher

Cross-platform filesystem monitoring based on \`notify\` library:

\`\`\`rust
use notify::{Watcher, RecursiveMode, Event};

let (tx, rx) = channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(path, RecursiveMode::Recursive)?;

for event in rx {
    match event {
        Ok(Event { kind: EventKind::Create(_), paths, .. }) => {
            handle_create(paths).await?;
        }
        Ok(Event { kind: EventKind::Modify(_), paths, .. }) => {
            handle_modify(paths).await?;
        }
        _ => {}
    }
}
\`\`\`

### Transfer Layer

Local transfers use file copying, remote transfers use QUIC protocol:

\`\`\`rust
// Local transfer
tokio::fs::copy(src, dest).await?;

// QUIC transfer
let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
let conn = endpoint.connect(addr, "turbosync")?.await?;
let mut stream = conn.open_uni().await?;
tokio::io::copy(&mut file, &mut stream).await?;
\`\`\`

## Performance Optimizations

- **Concurrent Scanning**: Process multiple directories concurrently using tokio
- **Incremental Sync**: Transfer only changed file blocks
- **Batch Operations**: Reduce I/O with batch database inserts
- **Memory Mapping**: Use mmap for reading large files`
      },
      cli: {
        title: 'CLI Commands',
        content: `Complete command-line tool supporting all sync operations.

## Basic Commands

### Initialize Configuration

Initialize on first run:

\`\`\`bash
# Initialize config file
tsync init

# View config path
tsync config path

# Edit config file
tsync config edit
\`\`\`

### Agent Management

\`\`\`bash
# Run in foreground (view logs)
tsync agent run

# Start in background
tsync agent start

# Stop service
tsync agent stop

# Restart service
tsync agent restart

# Check status
tsync agent status
\`\`\`

## Node Management

### Add Node

\`\`\`bash
# Add local node
tsync node add localhost 127.0.0.1:9527

# Add remote node
tsync node add nas 192.168.1.100:9527

# Add node with TLS enabled
tsync node add server example.com:9527 --tls

# Add node with alias
tsync node add prod-server 10.0.0.1:9527 --alias production
\`\`\`

### List Nodes

\`\`\`bash
# List all nodes
tsync node list

# List online nodes
tsync node list --online

# Show detailed info
tsync node list --verbose
\`\`\`

### Node Operations

\`\`\`bash
# Test node connection
tsync node ping nas

# View node details
tsync node info nas

# Update node address
tsync node update nas --address 192.168.1.101:9527

# Remove node
tsync node remove nas
\`\`\`

## Task Management

### Create Task

\`\`\`bash
# Basic syntax
tsync task create <source> <target-node>:<target-path>

# One-way sync
tsync task create ~/Documents nas:/backup/docs

# Bidirectional sync
tsync task create ~/code nas:/code --mode bidirectional

# Mirror mode (deletes extra files on target)
tsync task create ~/deploy server:/www --mode mirror

# Set filter rules
tsync task create ~/project nas:/project --ignore "*.log,node_modules/"

# Enable compression
tsync task create ~/text nas:/text --compress
\`\`\`

### Task List

\`\`\`bash
# List all tasks
tsync task list

# List active tasks
tsync task list --active

# List paused tasks
tsync task list --paused

# JSON format output
tsync task list --json
\`\`\`

### Task Operations

\`\`\`bash
# View task details
tsync task info <task-id>

# Enable task
tsync task enable <task-id>

# Pause task
tsync task pause <task-id>

# Resume task
tsync task resume <task-id>

# Remove task
tsync task remove <task-id>

# Force remove (including history data)
tsync task remove <task-id> --force
\`\`\`

## Sync Operations

### Manual Sync

\`\`\`bash
# Sync all tasks
tsync sync

# Sync specific task
tsync sync <task-id>

# Force full sync (ignore cache)
tsync sync <task-id> --full

# Execute once only
tsync sync <task-id> --once
\`\`\`

### Monitor Status

\`\`\`bash
# View all task statuses
tsync status

# Real-time monitoring
tsync monitor

# View transfer speed
tsync monitor --speed

# Export status report
tsync status --export report.json
\`\`\`

## Log Viewing

\`\`\`bash
# View recent logs
tsync log

# View specific task logs
tsync log --task <task-id>

# View error logs
tsync log --level error

# Follow logs in real-time
tsync log --follow

# Export logs
tsync log --export turbosync.log

# Clean old logs
tsync log clean --older-than 7d
\`\`\`

## Configuration Management

\`\`\`bash
# View configuration
tsync config get

# Set config item
tsync config set <key> <value>

# Common configs
tsync config set log.level debug
tsync config set transfer.concurrency 8
tsync config set transfer.chunk_size 8MB
tsync config set compress.enabled true

# Reset configuration
tsync config reset
\`\`\`

## Advanced Features

### Import/Export

\`\`\`bash
# Export configuration
tsync export --output backup.json

# Import configuration
tsync import --input backup.json

# Export specific task
tsync task export <task-id> --output task.json

# Import task
tsync task import --input task.json
\`\`\`

### Diagnostic Tools

\`\`\`bash
# Run diagnostics
tsync diagnose

# Check filesystem permissions
tsync diagnose permissions

# Check network connectivity
tsync diagnose network

# Generate diagnostic report
tsync diagnose --report
\`\`\`

### Database Maintenance

\`\`\`bash
# Database statistics
tsync db stats

# Clean cache
tsync db clean

# Optimize database
tsync db optimize

# Backup database
tsync db backup --output backup.db
\`\`\`

## Script Examples

### Automatic Backup Script

\`\`\`bash
#!/bin/bash
# backup.sh - Scheduled backup script

# Ensure Agent is running
tsync agent status || tsync agent start

# Execute all backup tasks
tsync sync --tag backup

# Check for errors
if [ $? -eq 0 ]; then
    echo "Backup completed successfully"
else
    echo "Backup failed" >&2
    exit 1
fi
\`\`\`

### Batch Create Tasks

\`\`\`bash
#!/bin/bash
# setup-sync.sh - Batch create sync tasks

declare -A tasks=(
    ["~/Documents"]="nas:/backup/docs"
    ["~/Pictures"]="nas:/backup/pics"
    ["~/Videos"]="nas:/backup/videos"
)

for source in "\${!tasks[@]}"; do
    target="\${tasks[$source]}"
    echo "Creating task: $source -> $target"
    tsync task create "$source" "$target" --mode one-way
done

echo "All tasks created. Starting sync..."
tsync sync
\`\`\``
      }
    }
  }

  const currentDocContent = docContent[lang][activeDoc]
  const filteredDocNav = docNav.filter(doc =>
    doc.title.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const features = lang === 'zh' ? [
    { title: '桌面 GUI', desc: 'Tauri + React 桌面控制台' },
    { title: '终端 TUI', desc: '中文默认，支持英文切换' },
    { title: '实时同步', desc: '基于文件系统监听' },
    { title: '增量传输', desc: '只传输变化的部分' },
    { title: 'SQLite 存储', desc: '轻量级本地数据库' },
    { title: '跨平台', desc: 'Linux / macOS / Windows' },
    { title: 'Rust 编写', desc: '高性能、内存安全' },
    { title: '双向同步', desc: '多节点互相同步' },
    { title: '冲突处理', desc: '智能合并冲突文件' },
    { title: '过滤规则', desc: '自定义忽略模式' },
    { title: 'CLI 命令', desc: '命令行完整控制' },
    { title: '开源免费', desc: 'MIT 协议' }
  ] : [
    { title: 'Desktop GUI', desc: 'Tauri + React console' },
    { title: 'Terminal TUI', desc: 'Bilingual support' },
    { title: 'Real-time Sync', desc: 'Filesystem watch' },
    { title: 'Incremental', desc: 'Delta transfer' },
    { title: 'SQLite', desc: 'Lightweight database' },
    { title: 'Cross-platform', desc: 'Linux / macOS / Windows' },
    { title: 'Rust', desc: 'Performance & Safety' },
    { title: 'Bidirectional', desc: 'Multi-node sync' },
    { title: 'Conflict', desc: 'Smart merge' },
    { title: 'Filters', desc: 'Custom ignore patterns' },
    { title: 'CLI', desc: 'Full control' },
    { title: 'Open Source', desc: 'MIT License' }
  ]

  const docs = lang === 'zh' ? [
    { title: '日常使用推荐', desc: '优先使用桌面 GUI 或终端 TUI。它们会自动初始化配置、连接本机 Agent，并提供节点、任务、同步和日志入口。' },
    { title: '后台同步服务', desc: 'turbosync-agent 负责扫描目录、监听文件变化、生成同步计划，并通过本地复制或 QUIC 传输文件。' },
    { title: '同步任务模型', desc: '一个任务由源目录、目标节点和目标路径组成。当前最稳定主线是源目录到目标目录的单向同步。' },
    { title: '服务器与 Homelab', desc: '远端机器只需要运行 Agent，然后在本机添加节点即可。适合 NAS、服务器备份和多设备目录同步。' }
  ] : [
    { title: 'Recommended daily flow', desc: 'Start with the desktop GUI or terminal TUI. They initialize config, connect to the local Agent, and expose nodes, tasks, sync, and logs.' },
    { title: 'Background sync service', desc: 'turbosync-agent scans folders, watches file changes, creates sync plans, and transfers files by local copy or QUIC.' },
    { title: 'Task model', desc: 'A task is defined by a source folder, a target node, and a target path. The most stable path today is source-to-target one-way sync.' },
    { title: 'Servers and Homelab', desc: 'Remote machines only need to run the Agent. Add them as nodes locally for NAS, server backup, and multi-device folder sync.' }
  ]

  const downloads = lang === 'zh' ? [
    { title: '桌面端', desc: '推荐给普通用户。图形化管理节点、任务和状态。', action: '查看桌面端下载' },
    { title: '命令行', desc: '适合服务器、脚本、CI 和自动化部署。', action: '查看 CLI 下载' },
    { title: '源码构建', desc: '适合开发者审计、贡献或二次开发。', action: '打开源码仓库' }
  ] : [
    { title: 'Desktop App', desc: 'Recommended for daily users. Manage nodes, tasks, and status visually.', action: 'View Desktop Download' },
    { title: 'Command Line', desc: 'For servers, scripts, CI, and automation.', action: 'View CLI Download' },
    { title: 'Build from Source', desc: 'For auditing, contribution, or customization.', action: 'Open Source Repo' }
  ]

  return (
    <div className="app">
      <header className="header">
        <button type="button" className="header-left" onClick={() => navigateTo('home')} aria-label={content.logo}>
          <img src={`${import.meta.env.BASE_URL}feisuo-mascot.svg`} alt="" className="mascot" />
          <div className="logo">{content.logo}</div>
        </button>
        <nav className="nav">
          <button type="button" onClick={() => navigateTo('home')}>{content.features}</button>
          <button type="button" onClick={() => navigateTo('docs')}>{content.docs}</button>
          <button type="button" onClick={() => navigateTo('download')}>{content.download}</button>
          <a href={REPO_URL} target="_blank" rel="noopener noreferrer">{content.github}</a>

          <div className="dropdown">
            <button type="button" onClick={() => setShowThemeMenu(!showThemeMenu)} className="icon-btn" aria-label={content.theme.auto}>
              {theme === 'dark' ? '🌙' : theme === 'light' ? '☀️' : '💻'}
            </button>
            {showThemeMenu && (
              <div className="dropdown-menu">
                <button type="button" onClick={() => handleThemeChange('light')}>☀️ {content.theme.light}</button>
                <button type="button" onClick={() => handleThemeChange('dark')}>🌙 {content.theme.dark}</button>
                <button type="button" onClick={() => handleThemeChange('auto')}>💻 {content.theme.auto}</button>
              </div>
            )}
          </div>

          <div className="dropdown">
            <button type="button" onClick={() => setShowLangMenu(!showLangMenu)} className="icon-btn" aria-label="Language">
              {lang === 'zh' ? '🇨🇳' : '🇬🇧'}
            </button>
            {showLangMenu && (
              <div className="dropdown-menu">
                <button type="button" onClick={() => handleLangChange('zh')}>🇨🇳 简体中文</button>
                <button type="button" onClick={() => handleLangChange('en')}>🇬🇧 English</button>
              </div>
            )}
          </div>
        </nav>
      </header>

      <main>
        {page === 'home' && (
          <>
            <section className="hero">
              <img src={`${import.meta.env.BASE_URL}feisuo-mascot.svg`} alt="飞梭吉祥物" className="hero-mascot" />
              <h1 className="hero-title">{content.heroTitle}</h1>
              <p className="hero-subtitle">{content.heroSubtitle}</p>
              <p className="hero-desc">{content.heroDesc}</p>
            </section>

            <section id="features" className="features" aria-label={content.features}>
              {features.map((feature) => (
                <div key={feature.title} className="feature-card">
                  <h3>{feature.title}</h3>
                  <p>{feature.desc}</p>
                </div>
              ))}
            </section>
          </>
        )}

        {page === 'docs' && (
          <section className="docs-layout">
            <aside className="docs-sidebar">
              <div className="docs-search">
                <input
                  type="text"
                  placeholder={lang === 'zh' ? '搜索文档...' : 'Search docs...'}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
              </div>
              <nav className="docs-nav">
                {filteredDocNav.map(item => (
                  <button
                    key={item.id}
                    type="button"
                    className={`docs-nav-item ${activeDoc === item.id ? 'active' : ''}`}
                    onClick={() => setActiveDoc(item.id)}
                  >
                    <span className="docs-nav-icon">{item.icon}</span>
                    <span className="docs-nav-title">{item.title}</span>
                  </button>
                ))}
              </nav>
            </aside>
            <article className="docs-content">
              <h1>{currentDocContent.title}</h1>
              <div className="markdown-body" dangerouslySetInnerHTML={{ __html: marked(currentDocContent.content) }} />
            </article>
          </section>
        )}

        {page === 'download' && (
          <section className="page-section">
            <div className="page-heading">
              <p className="eyebrow">Download</p>
              <h1>{content.downloadTitle}</h1>
              <p>{content.downloadDesc}</p>
            </div>
            <div className="download-grid">
              {downloads.map((item, index) => (
                <article key={item.title} className="download-card">
                  <h3>{item.title}</h3>
                  <p>{item.desc}</p>
                  <a href={index === 2 ? REPO_URL : `${REPO_URL}/releases`} target="_blank" rel="noopener noreferrer">
                    {item.action}
                  </a>
                </article>
              ))}
            </div>
          </section>
        )}
      </main>

      <footer className="footer">
        <p>{content.footer}</p>
      </footer>
    </div>
  )
}

export default App
