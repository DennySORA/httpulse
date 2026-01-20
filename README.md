# httpulse

![httpulse](docs/screenshot.png)

[繁體中文](docs/README.zh-TW.md) | [简体中文](docs/README.zh-CN.md) | [日本語](docs/README.ja.md)

> Real-time HTTP latency and network quality monitoring with an interactive terminal UI.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/DennySORA/httpulse/actions/workflows/ci.yml)
[![Release](https://github.com/DennySORA/httpulse/actions/workflows/release.yml/badge.svg)](https://github.com/DennySORA/httpulse/releases)

**httpulse** is a powerful network diagnostics tool that provides deep visibility into HTTP connection performance. Built in Rust for speed and reliability, it helps DevOps engineers, SREs, and developers identify latency bottlenecks and network issues in real-time.

## Quick Install

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

**Windows:**

Download [`httpulse-x86_64-pc-windows-msvc.zip`](https://github.com/DennySORA/httpulse/releases/latest/download/httpulse-x86_64-pc-windows-msvc.zip), extract, and run `httpulse.exe`.

See [Installation](#installation) for more options.

## Why httpulse?

- **Pinpoint Latency Issues** — Break down request time into DNS, TCP connect, TLS handshake, TTFB, and download phases
- **Compare Configurations** — Test HTTP/1.1 vs HTTP/2, TLS 1.2 vs 1.3, warm vs cold connections side-by-side
- **Monitor Multiple Endpoints** — Track several URLs simultaneously with independent metrics
- **Kernel-Level Insights** — Access TCP_INFO metrics (RTT, cwnd, retransmissions) on Linux for accurate diagnostics
- **Beautiful TUI** — Interactive terminal interface with real-time charts and organized metric categories

## Features

### Comprehensive Metrics

| Category | Metrics |
|----------|---------|
| **Latency** | DNS, TCP Connect, TLS Handshake, TTFB, Download, Total |
| **Quality** | RTT, RTT Variance, Jitter |
| **Reliability** | Retransmissions, Packet Reordering, Probe Loss Rate |
| **Throughput** | Goodput (Mbps), Bandwidth Utilization |
| **TCP State** | Congestion Window (cwnd), Slow-start Threshold (ssthresh) |

### Configurable Profiles

Mix and match HTTP versions, TLS versions, and connection modes:

```
h2+tls13+warm   # HTTP/2, TLS 1.3, connection reuse
h1+tls12+cold   # HTTP/1.1, TLS 1.2, fresh connection each probe
```

### Interactive TUI

- **Real-time Charts** — Visualize latency trends over time
- **Multiple View Modes** — Split, Chart-only, Metrics-only, Summary
- **Time Windows** — 1min, 5min, 15min, 60min aggregation
- **Built-in Glossary** — Learn what each metric means

## Installation

### Quick Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/DennySORA/httpulse/main/install.sh | bash
```

### Download Binary

Download pre-built binaries from [GitHub Releases](https://github.com/DennySORA/httpulse/releases):

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | `httpulse-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `httpulse-aarch64-unknown-linux-musl.tar.gz` |
| macOS | Intel | `httpulse-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `httpulse-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `httpulse-x86_64-pc-windows-msvc.zip` |

### Build from Source

```bash
git clone https://github.com/DennySORA/httpulse.git
cd httpulse
cargo build --release
# Binary: target/release/httpulse
```

### Cargo Install

```bash
cargo install --git https://github.com/DennySORA/httpulse.git
```

## Usage

### Basic Usage

```bash
# Monitor default target (google.com)
httpulse

# Monitor specific target
httpulse -t https://example.com

# Monitor multiple targets
httpulse -t https://api.example.com -t https://cdn.example.com -t https://db.example.com
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-t, --target <URL>` | Target URL to probe (repeatable) | `https://google.com` |
| `--refresh-hz <N>` | UI refresh rate in Hz | `10` |
| `--ebpf <MODE>` | eBPF mode: `off` \| `minimal` \| `full` | `off` |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Navigate targets |
| `Tab` | Cycle profiles |
| `[` / `]` | Switch metric category |
| `a` | Add target |
| `e` | Edit target |
| `d` | Delete target |
| `p` | Pause/Resume probing |
| `c` | Toggle compare mode |
| `g` | Cycle view mode |
| `w` | Cycle time window |
| `1-8` | Toggle chart metrics |
| `?` | Help |
| `G` | Glossary |
| `S` | Settings |
| `q` | Quit |

### Adding Targets

Press `a` and enter a URL with optional profile specs:

```
https://api.example.com h2+tls13+warm h1+tls12+cold
```

Profile format: `<http>+<tls>+<conn>`
- **HTTP**: `h1`, `h2`
- **TLS**: `tls12`, `tls13`
- **Connection**: `warm` (reuse), `cold` (fresh)

### Settings

Press `S` to configure:
- UI refresh rate
- Link capacity (for bandwidth utilization)
- Probe interval
- Timeout duration
- DNS timing toggle

## Understanding Metrics

### Statistics Format

Metrics display as **P50 / P99 / Mean**:
- **P50**: Median (50th percentile)
- **P99**: 99th percentile (worst 1%)
- **Mean**: Average

### Quick Reference

| Metric | Good | Warning | Critical |
|--------|------|---------|----------|
| Success Rate | ≥99% | 95-99% | <95% |
| Latency P99 | <100ms | 100-500ms | >500ms |
| Retransmissions | 0 | 1-3 | >3 |

### Platform Notes

- **Linux**: Full TCP_INFO support (cwnd, ssthresh, rtt, rttvar, retrans, reordering)
- **macOS/Windows**: Application-level metrics only

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          Main Thread                            │
│  ┌──────────┐    ┌───────────┐    ┌──────────────────────────┐ │
│  │ AppState │◄───│  UI Loop  │◄───│ crossbeam channel (rx)   │ │
│  └──────────┘    └───────────┘    └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                              ▲
              ┌───────────────────────────────┴─────────────────────────────┐
              │                                                             │
  ┌───────────┴────────────┐                          ┌─────────────────────┴───────────┐
  │    Worker Thread 1     │           ...            │       Worker Thread N           │
  │  ┌──────────────────┐  │                          │  ┌───────────────────────────┐  │
  │  │   ProbeClient    │  │                          │  │       ProbeClient         │  │
  │  │    (libcurl)     │  │                          │  │        (libcurl)          │  │
  │  └──────────────────┘  │                          │  └───────────────────────────┘  │
  └────────────────────────┘                          └─────────────────────────────────┘
```

## Requirements

- **OS**: Linux (full features), macOS, Windows
- **Dependencies**: libcurl with HTTP/2 support

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.
