# httpulse

![httpulse](screenshot.png)

[English](../README.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md)

> 实时 HTTP 延迟与网络质量监控工具，具备交互式终端界面。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml)
[![Release](https://github.com/DennySORA/httpulse/actions/workflows/release.yml/badge.svg)](https://github.com/DennySORA/httpulse/releases)

**httpulse** 是一款强大的网络诊断工具，能深入分析 HTTP 连接性能。使用 Rust 构建，兼具速度与稳定性，帮助 DevOps 工程师、SRE 和开发者实时识别延迟瓶颈与网络问题。

## 快速安装

**Linux / macOS：**
```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

**Windows：**

下载 [`httpulse-x86_64-pc-windows-msvc.zip`](https://github.com/DennySORA/httpulse/releases/latest/download/httpulse-x86_64-pc-windows-msvc.zip)，解压后运行 `httpulse.exe`。

详见 [安装](#安装) 章节。

## 为何选择 httpulse？

- **精准定位延迟问题** — 将请求时间拆解为 DNS、TCP 连接、TLS 握手、TTFB 和下载等阶段
- **比较不同配置** — 并排测试 HTTP/1.1 vs HTTP/2、TLS 1.2 vs 1.3、暖连接 vs 冷连接
- **同时监控多个端点** — 追踪多个 URL，各自独立统计指标
- **内核级洞察** — 在 Linux 上访问 TCP_INFO 指标（RTT、cwnd、重传次数），提供精确诊断
- **精美的 TUI** — 交互式终端界面，实时图表与分类指标一目了然

## 功能特色

### 完整指标

| 类别 | 指标 |
|------|------|
| **延迟** | DNS、TCP 连接、TLS 握手、TTFB、下载、总时间 |
| **质量** | RTT、RTT 方差、Jitter |
| **可靠性** | 重传次数、包乱序、探测失败率 |
| **吞吐量** | Goodput (Mbps)、带宽利用率 |
| **TCP 状态** | 拥塞窗口 (cwnd)、慢启动阈值 (ssthresh) |

### 可配置的配置文件

自由组合 HTTP 版本、TLS 版本和连接模式：

```
h2+tls13+warm   # HTTP/2, TLS 1.3, 连接重用
h1+tls12+cold   # HTTP/1.1, TLS 1.2, 每次探测建立新连接
```

### 交互式 TUI

- **实时图表** — 可视化延迟趋势
- **多种视图模式** — 分割、仅图表、仅指标、摘要
- **时间窗口** — 1 分钟、5 分钟、15 分钟、60 分钟聚合
- **内置术语表** — 了解每个指标的含义

## 安装

### 快速安装（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

### 下载可执行文件

从 [GitHub Releases](https://github.com/DennySORA/httpulse/releases) 下载预编译可执行文件：

| 平台 | 架构 | 下载文件 |
|------|------|----------|
| Linux | x86_64 | `httpulse-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `httpulse-aarch64-unknown-linux-musl.tar.gz` |
| macOS | Intel | `httpulse-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `httpulse-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `httpulse-x86_64-pc-windows-msvc.zip` |

### 从源码构建

```bash
git clone https://github.com/DennySORA/httpulse.git
cd httpulse
cargo build --release
# 可执行文件：target/release/httpulse
```

### Cargo 安装

```bash
cargo install --git https://github.com/DennySORA/httpulse.git
```

## 使用方式

### 基本用法

```bash
# 监控默认目标 (google.com)
httpulse

# 监控特定目标
httpulse -t https://example.com

# 监控多个目标
httpulse -t https://api.example.com -t https://cdn.example.com -t https://db.example.com
```

### 命令行选项

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `-t, --target <URL>` | 要探测的目标 URL（可重复） | `https://google.com` |
| `--refresh-hz <N>` | UI 刷新频率 (Hz) | `10` |
| `--ebpf <MODE>` | eBPF 模式：`off` \| `minimal` \| `full` | `off` |

### 键盘快捷键

| 按键 | 动作 |
|------|------|
| `j/k` 或 `↑/↓` | 切换目标 |
| `Tab` | 切换配置文件 |
| `[` / `]` | 切换指标类别 |
| `a` | 添加目标 |
| `e` | 编辑目标 |
| `d` | 删除目标 |
| `p` | 暂停/继续探测 |
| `c` | 切换比较模式 |
| `g` | 切换视图模式 |
| `w` | 切换时间窗口 |
| `1-8` | 切换图表指标 |
| `?` | 帮助 |
| `G` | 术语表 |
| `S` | 设置 |
| `q` | 退出 |

### 添加目标

按 `a` 并输入 URL，可选择性加入配置文件规格：

```
https://api.example.com h2+tls13+warm h1+tls12+cold
```

配置文件格式：`<http>+<tls>+<conn>`
- **HTTP**：`h1`、`h2`
- **TLS**：`tls12`、`tls13`
- **连接**：`warm`（重用）、`cold`（新建）

### 设置

按 `S` 可配置：
- UI 刷新频率
- 链路容量（用于计算带宽利用率）
- 探测间隔
- 超时时间
- DNS 计时开关

## 理解指标

### 统计格式

指标显示为 **P50 / P99 / Mean**：
- **P50**：中位数（第 50 百分位数）
- **P99**：第 99 百分位数（最差的 1%）
- **Mean**：平均值

### 快速参考

| 指标 | 良好 | 警告 | 严重 |
|------|------|------|------|
| 成功率 | ≥99% | 95-99% | <95% |
| 延迟 P99 | <100ms | 100-500ms | >500ms |
| 重传次数 | 0 | 1-3 | >3 |

### 平台说明

- **Linux**：完整 TCP_INFO 支持（cwnd、ssthresh、rtt、rttvar、retrans、reordering）
- **macOS/Windows**：仅应用层指标

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                            主线程                                │
│  ┌──────────┐    ┌───────────┐    ┌──────────────────────────┐ │
│  │ AppState │◄───│  UI 循环  │◄───│ crossbeam channel (rx)   │ │
│  └──────────┘    └───────────┘    └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                              ▲
              ┌───────────────────────────────┴─────────────────────────────┐
              │                                                             │
  ┌───────────┴────────────┐                          ┌─────────────────────┴───────────┐
  │       工作线程 1        │           ...            │          工作线程 N             │
  │  ┌──────────────────┐  │                          │  ┌───────────────────────────┐  │
  │  │   ProbeClient    │  │                          │  │       ProbeClient         │  │
  │  │    (libcurl)     │  │                          │  │        (libcurl)          │  │
  │  └──────────────────┘  │                          │  └───────────────────────────┘  │
  └────────────────────────┘                          └─────────────────────────────────┘
```

## 系统需求

- **操作系统**：Linux（完整功能）、macOS、Windows
- **依赖项**：支持 HTTP/2 的 libcurl

## 许可证

MIT 许可证 - 详见 [LICENSE](../LICENSE)。

## 贡献

欢迎贡献！请随时提交 Issue 和 Pull Request。
