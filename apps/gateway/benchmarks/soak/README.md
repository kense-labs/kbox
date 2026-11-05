# Soak result schema

## 概述

本目录保存 apps/gateway/benchmarks/soak 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

JSON files under `results/` follow this shape. Smoke artifacts use
`envelope_status: "smoke-only"`. Published dedicated-hardware envelopes must
set `envelope_status: "published"` and fill every hardware pin field.

```json
{
  "schema": "kense.gateway.soak-result.v1",
  "envelope_status": "smoke-only",
  "profile": "http-json",
  "duration_secs": 30,
  "concurrency": 16,
  "hardware": {
    "host": null,
    "cpu_model": null,
    "cpu_cores": null,
    "memory_gb": null,
    "os": "windows",
    "kernel": null
  },
  "gateway": {
    "version": null,
    "git_sha": null,
    "features": [],
    "bin": "target/debug/krun-gateway"
  },
  "requests": { "ok": 0, "err": 0 },
  "rss_kb": { "min": null, "max": null, "growth": null, "samples": 0 },
  "pass": true,
  "fail_reasons": []
}
```

Do not treat smoke-only JSON as a capacity promise in README or marketing.
