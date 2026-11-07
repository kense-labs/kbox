# kbox 常用服务部署 E2E

验证 `kbox run -d -p …` 部署常见 OCI 服务后，从**宿主**可达并完成最小业务探测。

## 环境

| 项 | 值 |
| --- | --- |
| 平台 | macOS arm64（HVF） |
| kbox | ≥ 3.4.3 |
| 隔离 | 每次运行独立 `K_HOME`（`$E2E_OUT/k-home`） |
| 网络 | 需能拉取 Docker Hub / GHCR 镜像 |

## 用例

| ID | 镜像 | 功能断言 |
| --- | --- | --- |
| S-NGINX | `docker.io/library/nginx:alpine` | bind-mount 自定义 `index.html`；宿主 `GET /` 返回该内容 |
| S-REDIS | `docker.io/library/redis:7-alpine` | 宿主 RESP `SET/GET/DEL`；AOF + 同卷重启后 `GET` 仍命中 |
| S-PG | `docker.io/library/postgres:16-alpine` | 宿主 TCP；客体内 `CREATE/INSERT/SELECT`；同卷重启后行仍在 |
| S-RUSTFS | `docker.io/rustfs/rustfs:latest` | 宿主 `/health` + SigV4 `CreateBucket/PutObject/GetObject` |

## 通过标准

- box `running`，`kbox port` 显示映射
- 宿主探测成功（或客体内等价探测并记录）
- `kbox rm -f` 后无残留 running box
- 结果写入 `results.json`；**不**记录真实密钥明文（RustFS 用合成测试凭据）

## 运行

```sh
TIER=all ./scripts/e2e-services/run.sh
# 或
TIER=nginx,redis,postgres,rustfs ./scripts/e2e-services/run.sh
```
