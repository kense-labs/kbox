# TLS Test Fixtures

## 概述

本目录保存 apps/gateway/tests/fixtures/tls 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

These certificates and private keys are public, non-production fixtures used
only by the managed-listener integration tests. Revisions 1 and 2 use separate
test certificate authorities so a real client can prove that a same-address
Gateway reload stopped serving the superseded certificate.
