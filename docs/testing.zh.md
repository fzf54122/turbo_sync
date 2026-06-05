# TurboSync 开发自测

这份文档面向开发者和发布前验收。普通用户看 `docs/user-guide.zh.md`。

自测分三类：

1. 本地 → 本地：验证最常用的文件夹复制。
2. 本地 → 远程：验证两个 Agent 之间的 QUIC 传输。
3. 双向同步：验证两端互相补齐文件。

下面命令默认在仓库根目录执行。

## 0. 基础检查

```bash
make check
```

如果环境不允许绑定 `127.0.0.1` 端口，transport 测试可能失败；换普通本机终端运行即可。

## 1. 本地 → 本地

目标：把本机目录 `/tmp/turbosync-local/src` 同步到 `/tmp/turbosync-local/dst`。

准备：

```bash
rm -rf /tmp/turbosync-local
mkdir -p /tmp/turbosync-local/src /tmp/turbosync-local/dst
echo "hello" > /tmp/turbosync-local/src/hello.txt

export TURBOSYNC_CONFIG=/tmp/turbosync-local/config.toml
export TURBOSYNC_DB=/tmp/turbosync-local/turbosync.db
export TSYNC="cargo run -p turbosync-cli --"
```

启动控制台：

```bash
$TSYNC dashboard
```

第一次启动会自动初始化并启动本地后台同步服务。

另开一个终端，添加本机节点和任务：

```bash
export TURBOSYNC_CONFIG=/tmp/turbosync-local/config.toml
export TURBOSYNC_DB=/tmp/turbosync-local/turbosync.db
export TSYNC="cargo run -p turbosync-cli --"

$TSYNC node add local 127.0.0.1:38745
LOCAL_NODE_ID=<node add 输出的节点 ID>

$TSYNC task add local-copy \
  --source /tmp/turbosync-local/src \
  --target-node "$LOCAL_NODE_ID" \
  --target-path /tmp/turbosync-local/dst
```

回到控制台按 `s` 同步。

检查：

```bash
cat /tmp/turbosync-local/dst/hello.txt
```

预期输出：

```text
hello
```

再测更新和删除：

```bash
echo "updated" > /tmp/turbosync-local/src/hello.txt
echo "remove me" > /tmp/turbosync-local/src/delete-me.txt
```

回到控制台按 `s`，确认目标目录出现 `delete-me.txt`。

然后删除源文件：

```bash
rm /tmp/turbosync-local/src/delete-me.txt
```

回到控制台再按 `s`，确认目标目录里的 `delete-me.txt` 也被删除。

## 2. 本地 → 远程

目标：在一台机器上模拟两台设备。

- local Agent：控制 API `127.0.0.1:38745`，传输端口 `127.0.0.1:38746`
- remote Agent：控制 API `127.0.0.1:39745`，传输端口 `127.0.0.1:39746`

准备目录：

```bash
rm -rf /tmp/turbosync-remote
mkdir -p /tmp/turbosync-remote/local-src
mkdir -p /tmp/turbosync-remote/remote-dst
echo "remote hello" > /tmp/turbosync-remote/local-src/hello.txt
```

初始化 local：

```bash
TURBOSYNC_CONFIG=/tmp/turbosync-remote/local.toml \
TURBOSYNC_DB=/tmp/turbosync-remote/local.db \
cargo run -p turbosync-cli -- init
```

初始化 remote：

```bash
TURBOSYNC_CONFIG=/tmp/turbosync-remote/remote.toml \
TURBOSYNC_DB=/tmp/turbosync-remote/remote.db \
cargo run -p turbosync-cli -- init
```

编辑 `/tmp/turbosync-remote/remote.toml`，把端口改成：

```toml
agent_addr = "127.0.0.1:39745"
transport_addr = "127.0.0.1:39746"
```

启动两个 Agent：

```bash
TURBOSYNC_CONFIG=/tmp/turbosync-remote/local.toml \
TURBOSYNC_DB=/tmp/turbosync-remote/local.db \
cargo run -p turbosync-cli -- agent run
```

另开终端：

```bash
TURBOSYNC_CONFIG=/tmp/turbosync-remote/remote.toml \
TURBOSYNC_DB=/tmp/turbosync-remote/remote.db \
cargo run -p turbosync-cli -- agent run
```

在 local 侧添加远端节点和任务：

```bash
export TURBOSYNC_CONFIG=/tmp/turbosync-remote/local.toml
export TURBOSYNC_DB=/tmp/turbosync-remote/local.db
export TSYNC="cargo run -p turbosync-cli --"

REMOTE_FP=$(grep '^cert_fingerprint' /tmp/turbosync-remote/remote.toml | cut -d '"' -f 2)

$TSYNC node add remote 127.0.0.1:39746 --cert-fingerprint "$REMOTE_FP"
REMOTE_NODE_ID=<node add 输出的节点 ID>

$TSYNC task add remote-copy \
  --source /tmp/turbosync-remote/local-src \
  --target-node "$REMOTE_NODE_ID" \
  --target-path /tmp/turbosync-remote/remote-dst

TASK_ID=<task add 输出的任务 ID>
$TSYNC sync "$TASK_ID"
```

检查：

```bash
cat /tmp/turbosync-remote/remote-dst/hello.txt
```

预期输出：

```text
remote hello
```

## 3. 双向同步

目标：两边各有一个文件，同步后两边都拥有完整内容。

本地双向最容易验证：

```bash
rm -rf /tmp/turbosync-two-way
mkdir -p /tmp/turbosync-two-way/a /tmp/turbosync-two-way/b
echo "from a" > /tmp/turbosync-two-way/a/a.txt
echo "from b" > /tmp/turbosync-two-way/b/b.txt

export TURBOSYNC_CONFIG=/tmp/turbosync-two-way/config.toml
export TURBOSYNC_DB=/tmp/turbosync-two-way/turbosync.db
export TSYNC="cargo run -p turbosync-cli --"

$TSYNC dashboard
```

另开终端添加本机节点和双向任务：

```bash
export TURBOSYNC_CONFIG=/tmp/turbosync-two-way/config.toml
export TURBOSYNC_DB=/tmp/turbosync-two-way/turbosync.db
export TSYNC="cargo run -p turbosync-cli --"

$TSYNC node add local 127.0.0.1:38745
LOCAL_NODE_ID=<node add 输出的节点 ID>

$TSYNC task add two-way \
  --source /tmp/turbosync-two-way/a \
  --target-node "$LOCAL_NODE_ID" \
  --target-path /tmp/turbosync-two-way/b \
  --direction two_way
```

回到控制台按 `s` 同步。

检查：

```bash
cat /tmp/turbosync-two-way/a/b.txt
cat /tmp/turbosync-two-way/b/a.txt
```

预期：

- `a/b.txt` 内容是 `from b`
- `b/a.txt` 内容是 `from a`

## 4. TUI 快速验收

启动：

```bash
tsync dashboard
```

检查点：

- 默认中文，按 `l` 可切换 English。
- `dashboard` 自动启动的后台同步服务，会在退出控制台时关闭。
- 左侧能看到同步任务。
- 右侧能看到“源目录 → 目标目录”。
- 按 `s` 同步时不会重复启动多个同步。
- 按 `w` / `x` 可以开启或停止监听。
- 按 `f` 可以查看文件索引。
- 按 `?` 可以查看帮助。

## 5. 清理

```bash
rm -rf /tmp/turbosync-local
rm -rf /tmp/turbosync-remote
rm -rf /tmp/turbosync-two-way
```
