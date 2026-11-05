# Plugin v3 TUF Fixture

## 概述

本目录保存 crates/use/extension/fixtures/registry/plugin-v3 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

This is deterministic test metadata for the complete `acme/research` package
fixture. The signing seed is public test data and the long expiration exists
only to make interoperability tests independent of wall-clock time.

Never use this root, key, signature, or expiration policy for a deployed
registry.
