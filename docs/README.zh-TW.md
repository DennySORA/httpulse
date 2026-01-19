# httpulse

![httpulse](screenshot.png)

[English](../README.md) | [简体中文](README.zh-CN.md) | [日本語](README.ja.md)

> 即時 HTTP 延遲與網路品質監控工具，具備互動式終端介面。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml)
[![Release](https://github.com/DennySORA/httpulse/actions/workflows/release.yml/badge.svg)](https://github.com/DennySORA/httpulse/releases)

**httpulse** 是一款強大的網路診斷工具，能深入分析 HTTP 連線效能。使用 Rust 建構，兼具速度與穩定性，協助 DevOps 工程師、SRE 和開發者即時識別延遲瓶頸與網路問題。

## 快速安裝

**Linux / macOS：**
```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

**使用 wget：**
```bash
wget -qO- https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

詳見 [安裝](#安裝) 章節。

## 為何選擇 httpulse？

- **精準定位延遲問題** — 將請求時間拆解為 DNS、TCP 連線、TLS 握手、TTFB 和下載等階段
- **比較不同配置** — 並排測試 HTTP/1.1 vs HTTP/2、TLS 1.2 vs 1.3、暖連線 vs 冷連線
- **同時監控多個端點** — 追蹤多個 URL，各自獨立統計指標
- **核心層級洞察** — 在 Linux 上存取 TCP_INFO 指標（RTT、cwnd、重傳次數），提供精確診斷
- **精美的 TUI** — 互動式終端介面，即時圖表與分類指標一目了然

## 功能特色

### 完整指標

| 類別 | 指標 |
|------|------|
| **延遲** | DNS、TCP 連線、TLS 握手、TTFB、下載、總時間 |
| **品質** | RTT、RTT 變異數、Jitter |
| **可靠性** | 重傳次數、封包亂序、探測失敗率 |
| **吞吐量** | Goodput (Mbps)、頻寬使用率 |
| **TCP 狀態** | 擁塞視窗 (cwnd)、慢啟動閾值 (ssthresh) |

### 可配置的設定檔

自由組合 HTTP 版本、TLS 版本和連線模式：

```
h2+tls13+warm   # HTTP/2, TLS 1.3, 連線重用
h1+tls12+cold   # HTTP/1.1, TLS 1.2, 每次探測建立新連線
```

### 互動式 TUI

- **即時圖表** — 視覺化延遲趨勢
- **多種檢視模式** — 分割、僅圖表、僅指標、摘要
- **時間視窗** — 1 分鐘、5 分鐘、15 分鐘、60 分鐘聚合
- **內建術語表** — 了解每個指標的含義

## 安裝

### 快速安裝（推薦）

```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

### 下載執行檔

從 [GitHub Releases](https://github.com/DennySORA/httpulse/releases) 下載預編譯執行檔：

| 平台 | 架構 | 下載檔案 |
|------|------|----------|
| Linux | x86_64 | `httpulse-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `httpulse-aarch64-unknown-linux-musl.tar.gz` |
| macOS | Intel | `httpulse-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `httpulse-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `httpulse-x86_64-pc-windows-msvc.zip` |

### 從原始碼建構

```bash
git clone https://github.com/DennySORA/httpulse.git
cd httpulse
cargo build --release
# 執行檔：target/release/httpulse
```

### Cargo 安裝

```bash
cargo install --git https://github.com/DennySORA/httpulse.git
```

## 使用方式

### 基本用法

```bash
# 監控預設目標 (google.com)
httpulse

# 監控特定目標
httpulse -t https://example.com

# 監控多個目標
httpulse -t https://api.example.com -t https://cdn.example.com -t https://db.example.com
```

### 命令列選項

| 選項 | 說明 | 預設值 |
|------|------|--------|
| `-t, --target <URL>` | 要探測的目標 URL（可重複） | `https://google.com` |
| `--refresh-hz <N>` | UI 更新頻率 (Hz) | `10` |
| `--ebpf <MODE>` | eBPF 模式：`off` \| `minimal` \| `full` | `off` |

### 鍵盤快捷鍵

| 按鍵 | 動作 |
|------|------|
| `j/k` 或 `↑/↓` | 切換目標 |
| `Tab` | 切換設定檔 |
| `[` / `]` | 切換指標類別 |
| `a` | 新增目標 |
| `e` | 編輯目標 |
| `d` | 刪除目標 |
| `p` | 暫停/繼續探測 |
| `c` | 切換比較模式 |
| `g` | 切換檢視模式 |
| `w` | 切換時間視窗 |
| `1-8` | 切換圖表指標 |
| `?` | 說明 |
| `G` | 術語表 |
| `S` | 設定 |
| `q` | 離開 |

### 新增目標

按 `a` 並輸入 URL，可選擇性加入設定檔規格：

```
https://api.example.com h2+tls13+warm h1+tls12+cold
```

設定檔格式：`<http>+<tls>+<conn>`
- **HTTP**：`h1`、`h2`
- **TLS**：`tls12`、`tls13`
- **連線**：`warm`（重用）、`cold`（新建）

### 設定

按 `S` 可配置：
- UI 更新頻率
- 連結容量（用於計算頻寬使用率）
- 探測間隔
- 逾時時間
- DNS 計時開關

## 理解指標

### 統計格式

指標顯示為 **P50 / P99 / Mean**：
- **P50**：中位數（第 50 百分位數）
- **P99**：第 99 百分位數（最差的 1%）
- **Mean**：平均值

### 快速參考

| 指標 | 良好 | 警告 | 嚴重 |
|------|------|------|------|
| 成功率 | ≥99% | 95-99% | <95% |
| 延遲 P99 | <100ms | 100-500ms | >500ms |
| 重傳次數 | 0 | 1-3 | >3 |

### 平台說明

- **Linux**：完整 TCP_INFO 支援（cwnd、ssthresh、rtt、rttvar、retrans、reordering）
- **macOS/Windows**：僅應用層指標

## 架構

```
┌─────────────────────────────────────────────────────────────────┐
│                           主執行緒                               │
│  ┌──────────┐    ┌───────────┐    ┌──────────────────────────┐ │
│  │ AppState │◄───│  UI 迴圈  │◄───│ crossbeam channel (rx)   │ │
│  └──────────┘    └───────────┘    └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                              ▲
              ┌───────────────────────────────┴─────────────────────────────┐
              │                                                             │
  ┌───────────┴────────────┐                          ┌─────────────────────┴───────────┐
  │      工作執行緒 1       │           ...            │         工作執行緒 N            │
  │  ┌──────────────────┐  │                          │  ┌───────────────────────────┐  │
  │  │   ProbeClient    │  │                          │  │       ProbeClient         │  │
  │  │    (libcurl)     │  │                          │  │        (libcurl)          │  │
  │  └──────────────────┘  │                          │  └───────────────────────────┘  │
  └────────────────────────┘                          └─────────────────────────────────┘
```

## 系統需求

- **作業系統**：Linux（完整功能）、macOS、Windows
- **相依套件**：支援 HTTP/2 的 libcurl

## 授權條款

MIT 授權條款 - 詳見 [LICENSE](../LICENSE)。

## 貢獻

歡迎貢獻！請隨時提交 Issue 和 Pull Request。
