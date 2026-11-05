# AI benchmark TLS material

## 概述

本目录保存 apps/gateway/benchmarks/ai-gateway-comparison/certs 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

`bench.crt` / `bench.key` are local self-signed fixtures copied from
`tests/fixtures/tls/revision-1.*` for TLS-termination lanes
(`transport-tls-http1`, `transport-tls-http2`).

Load clients must use `--insecure-tls`. Do not use these files for production.
