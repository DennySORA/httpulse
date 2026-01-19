# httpulse

![httpulse](screenshot.png)

[English](../README.md) | [繁體中文](README.zh-TW.md) | [简体中文](README.zh-CN.md)

> リアルタイム HTTP レイテンシとネットワーク品質監視ツール、インタラクティブなターミナル UI 付き。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml)
[![Release](https://github.com/DennySORA/httpulse/actions/workflows/release.yml/badge.svg)](https://github.com/DennySORA/httpulse/releases)

**httpulse** は HTTP 接続パフォーマンスを深く分析する強力なネットワーク診断ツールです。Rust で構築され、速度と信頼性を兼ね備え、DevOps エンジニア、SRE、開発者がリアルタイムでレイテンシのボトルネックやネットワーク問題を特定するのを支援します。

## クイックインストール

**Linux / macOS：**
```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

**wget を使用：**
```bash
wget -qO- https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

詳細は [インストール](#インストール) セクションを参照してください。

## なぜ httpulse？

- **レイテンシ問題を正確に特定** — リクエスト時間を DNS、TCP 接続、TLS ハンドシェイク、TTFB、ダウンロードなどのフェーズに分解
- **異なる設定を比較** — HTTP/1.1 vs HTTP/2、TLS 1.2 vs 1.3、ウォーム接続 vs コールド接続を並べてテスト
- **複数のエンドポイントを同時監視** — 複数の URL を追跡し、それぞれ独立したメトリクスを収集
- **カーネルレベルのインサイト** — Linux で TCP_INFO メトリクス（RTT、cwnd、再送回数）にアクセスし、正確な診断を提供
- **美しい TUI** — インタラクティブなターミナルインターフェース、リアルタイムチャートと分類されたメトリクス

## 機能

### 包括的なメトリクス

| カテゴリ | メトリクス |
|----------|------------|
| **レイテンシ** | DNS、TCP 接続、TLS ハンドシェイク、TTFB、ダウンロード、合計 |
| **品質** | RTT、RTT 分散、Jitter |
| **信頼性** | 再送回数、パケット順序乱れ、プローブ失敗率 |
| **スループット** | Goodput (Mbps)、帯域幅使用率 |
| **TCP 状態** | 輻輳ウィンドウ (cwnd)、スロースタート閾値 (ssthresh) |

### 設定可能なプロファイル

HTTP バージョン、TLS バージョン、接続モードを自由に組み合わせ：

```
h2+tls13+warm   # HTTP/2, TLS 1.3, 接続再利用
h1+tls12+cold   # HTTP/1.1, TLS 1.2, 毎回新規接続
```

### インタラクティブ TUI

- **リアルタイムチャート** — レイテンシトレンドを可視化
- **複数のビューモード** — 分割、チャートのみ、メトリクスのみ、サマリー
- **時間ウィンドウ** — 1 分、5 分、15 分、60 分の集計
- **内蔵用語集** — 各メトリクスの意味を学習

## インストール

### クイックインストール（推奨）

```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

### バイナリダウンロード

[GitHub Releases](https://github.com/DennySORA/httpulse/releases) からプリビルドバイナリをダウンロード：

| プラットフォーム | アーキテクチャ | ダウンロード |
|------------------|----------------|--------------|
| Linux | x86_64 | `httpulse-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `httpulse-aarch64-unknown-linux-musl.tar.gz` |
| macOS | Intel | `httpulse-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `httpulse-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `httpulse-x86_64-pc-windows-msvc.zip` |

### ソースからビルド

```bash
git clone https://github.com/DennySORA/httpulse.git
cd httpulse
cargo build --release
# バイナリ：target/release/httpulse
```

### Cargo インストール

```bash
cargo install --git https://github.com/DennySORA/httpulse.git
```

## 使用方法

### 基本的な使用法

```bash
# デフォルトターゲット (google.com) を監視
httpulse

# 特定のターゲットを監視
httpulse -t https://example.com

# 複数のターゲットを監視
httpulse -t https://api.example.com -t https://cdn.example.com -t https://db.example.com
```

### コマンドラインオプション

| オプション | 説明 | デフォルト |
|------------|------|------------|
| `-t, --target <URL>` | プローブするターゲット URL（繰り返し可能） | `https://google.com` |
| `--refresh-hz <N>` | UI リフレッシュレート (Hz) | `10` |
| `--ebpf <MODE>` | eBPF モード：`off` \| `minimal` \| `full` | `off` |

### キーボードショートカット

| キー | アクション |
|------|------------|
| `j/k` または `↑/↓` | ターゲットを切り替え |
| `Tab` | プロファイルを切り替え |
| `[` / `]` | メトリクスカテゴリを切り替え |
| `a` | ターゲットを追加 |
| `e` | ターゲットを編集 |
| `d` | ターゲットを削除 |
| `p` | プローブを一時停止/再開 |
| `c` | 比較モードを切り替え |
| `g` | ビューモードを切り替え |
| `w` | 時間ウィンドウを切り替え |
| `1-8` | チャートメトリクスを切り替え |
| `?` | ヘルプ |
| `G` | 用語集 |
| `S` | 設定 |
| `q` | 終了 |

### ターゲットの追加

`a` を押して URL を入力、オプションでプロファイル指定を追加：

```
https://api.example.com h2+tls13+warm h1+tls12+cold
```

プロファイル形式：`<http>+<tls>+<conn>`
- **HTTP**：`h1`、`h2`
- **TLS**：`tls12`、`tls13`
- **接続**：`warm`（再利用）、`cold`（新規）

### 設定

`S` を押して設定：
- UI リフレッシュレート
- リンク容量（帯域幅使用率計算用）
- プローブ間隔
- タイムアウト時間
- DNS タイミング切り替え

## メトリクスの理解

### 統計形式

メトリクスは **P50 / P99 / Mean** で表示：
- **P50**：中央値（50 パーセンタイル）
- **P99**：99 パーセンタイル（最悪の 1%）
- **Mean**：平均値

### クイックリファレンス

| メトリクス | 良好 | 警告 | 重大 |
|------------|------|------|------|
| 成功率 | ≥99% | 95-99% | <95% |
| レイテンシ P99 | <100ms | 100-500ms | >500ms |
| 再送回数 | 0 | 1-3 | >3 |

### プラットフォームノート

- **Linux**：完全な TCP_INFO サポート（cwnd、ssthresh、rtt、rttvar、retrans、reordering）
- **macOS/Windows**：アプリケーション層メトリクスのみ

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                         メインスレッド                           │
│  ┌──────────┐    ┌───────────┐    ┌──────────────────────────┐ │
│  │ AppState │◄───│ UI ループ │◄───│ crossbeam channel (rx)   │ │
│  └──────────┘    └───────────┘    └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                              ▲
              ┌───────────────────────────────┴─────────────────────────────┐
              │                                                             │
  ┌───────────┴────────────┐                          ┌─────────────────────┴───────────┐
  │   ワーカースレッド 1    │           ...            │       ワーカースレッド N         │
  │  ┌──────────────────┐  │                          │  ┌───────────────────────────┐  │
  │  │   ProbeClient    │  │                          │  │       ProbeClient         │  │
  │  │    (libcurl)     │  │                          │  │        (libcurl)          │  │
  │  └──────────────────┘  │                          │  └───────────────────────────┘  │
  └────────────────────────┘                          └─────────────────────────────────┘
```

## システム要件

- **OS**：Linux（フル機能）、macOS、Windows
- **依存関係**：HTTP/2 サポート付き libcurl

## ライセンス

MIT ライセンス - 詳細は [LICENSE](../LICENSE) を参照。

## コントリビューション

コントリビューション歓迎！Issue や Pull Request をお気軽にお送りください。
