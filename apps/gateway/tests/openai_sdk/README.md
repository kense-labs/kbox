# Official OpenAI SDK conformance

## 概述

本目录保存 apps/gateway/tests/openai_sdk 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

This suite runs the pinned official Python SDK against a real
`krun-gateway` process. It applies a complete inference policy through the
native managed snapshot API and uses a controllable HTTP/SSE upstream.

The cases cover:

- the complete Models, Chat Completions, Completions, and Embeddings endpoint
  matrix;
- typed non-streaming responses and the SDK-default base64 embedding path;
- stable authentication and grant error parsing;
- request rewriting and credential stripping;
- chat and legacy completion SSE framing, final usage chunks, and `[DONE]`
  termination while the upstream remains open;
- explicit SDK stream close and cancelled asynchronous consumers;
- graceful completion inside the configured drain deadline; and
- forced termination at a zero-second drain deadline.

Run it from the Krun repository root:

```bash
cargo build --locked -p krun-gateway --bin krun-gateway
python -m pip install --requirement apps/gateway/tests/openai_sdk/requirements.txt
python apps/gateway/tests/openai_sdk/test_conformance.py
```

The same commands work on Windows and automatically select
`target/debug/krun-gateway.exe`. To exercise a binary at another location, set
`KRUN_GATEWAY_BINARY` first:

```powershell
$env:KRUN_GATEWAY_BINARY = (Resolve-Path target/debug/krun-gateway.exe).Path
python tests/openai_sdk/test_conformance.py
```
