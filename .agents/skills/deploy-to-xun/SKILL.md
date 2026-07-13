---
name: deploy-to-xun
description: |
  将 yggdrasil 部署/更新到 xun 服务器时使用。本地 arm64 用 Docker Rosetta 构建
  x86 镜像，导出传输到 xun（Podman）后用 docker compose 运行，复用现有 nginx-proxy
  自动签发 HTTPS。触发关键词："部署"、"deploy"、"发布到 xun"、"更新 xun"、"上线"。
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - AskUserQuestion
  - Agent
metadata:
  trigger: 部署到 xun、更新线上、Docker 构建 x86 镜像、docker compose 上线
---

# Deploy to Xun: 部署/更新 yggdrasil 到 xun 服务器

从本地 arm64 构建器构建 `linux/amd64` 镜像，传输到 xun（Podman 5.8.2 / Rocky x86_64）并以 docker compose 运行。复用现有 `nginx-proxy` + `acme-companion` 做前置反代，自动签发 Let's Encrypt 证书。

## 目标环境速查

| 项 | 值 |
|---|---|
| 服务器别名 | `xun`（ssh 直接连） |
| OS / 架构 | Rocky Linux x86_64 |
| 容器运行时 | **Podman 5.8.2**（`docker` 是 podman 别名，rootful） |
| 默认登录 shell | **fish**（关键陷阱，见下） |
| socket 路径 | `/run/podman/podman.sock`（`root:root` 660，不是 `/var/run/docker.sock`） |
| 部署目录 | `/root/docker/yggdrasil/` |
| 前置代理 | 已有 `nginx-proxy` + `acme-companion`，占 80/443 |
| 代理网络 | `nginx-proxy`（bridge），容器设 `VIRTUAL_HOST` 即被自动发现 |
| 域名 | `rua.plus`（IPv4 已指向 xun，acme HTTP-01 可签） |
| 现有占用 | mimo-blog（xunrua.top）、frps 等，**勿动** |

## 本地构建机要求

- Apple Silicon arm64 + Docker Desktop
- **必须开启 Rosetta**：`UseVirtualizationFrameworkRosetta: true`（Docker Desktop 设置）
  - 检查：`grep UseVirtualizationFrameworkRosetta ~/Library/Group\ Containers/group.com.docker/settings-store.json`
  - 改完需重启 Docker Desktop
- 为什么必须 Rosetta：QEMU 用户态模拟跑 `rustc` 会 SIGSEGV（`qemu: uncaught target signal 11`），Rosetta 转译稳定且快 5-10 倍

## fish 陷阱（最重要的约定）

xun 的默认登录 shell 是 **fish**，不是 bash。通过 `ssh xun '...'` 执行的命令在 fish 里解析，会撞上：

| bash 语法 | fish 报错 | 解决 |
|---|---|---|
| `VAR=$(cmd)` | `Unsupported use of '='` | 用 `set VAR (cmd)`，或避免在服务器端赋值 |
| `for x in a b; do ...; done` | `Missing end to balance this for loop` | 用 `for x in a b; ...; end`，或**逐条单独 ssh 执行** |
| `echo $?` | `$? is not the exit status` | 用 `$status`，或别打印退出码 |
| `&&` 链带赋值 | 中途失败静默 | **拆成多条独立 ssh 命令**，每条单独验证 |

**最稳的做法：避免在 ssh 命令里写 shell 逻辑。** 每条 `ssh xun 'cmd'` 只跑一条 podman/compose 命令，结果用 `echo` 或退出码判断。需要循环时在本地跑 bash，循环体内逐条 ssh。

## 构建（本地 arm64 → linux/amd64）

### 主应用镜像

```bash
docker buildx build --platform linux/amd64 --load -t localhost/yggdrasil:latest .
```

- Dockerfile 用 `dpkg --print-architecture` 检测架构，amd64 腿原生构建 `x86_64-unknown-linux-musl` 静态二进制
- 首次约 15-30 分钟（Rosetta 下 cargo 全量编译），有 buildkit 缓存后分钟级
- 产物 `localhost/yggdrasil:latest`，scratch 运行时层约 16MB

### 5 个 Code Runner 沙箱镜像

runner Dockerfile `FROM yggdrasil-runner-base:latest`（无 `localhost/` 前缀），所以必须**先建 base 再建子镜像**：

```bash
# 1. base 先用 localhost/ 前缀建
docker buildx build --platform linux/amd64 --load \
  -t localhost/yggdrasil-runner-base:latest docker/runner-base
# 2. 再 tag 一个无前缀名，让子镜像 FROM 能解析
docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest
# 3. 4 个子镜像（它们 FROM yggdrasil-runner-base:latest）
for img in python node go rust; do
  docker buildx build --platform linux/amd64 --load \
    -t localhost/yggdrasil-runner-$img:latest docker/runner-$img
  docker tag localhost/yggdrasil-runner-$img:latest yggdrasil-runner-$img:latest
done
```

> **buildx v0.35 命名 bug**：`-t localhost/yggdrasil-runner-python:latest` 有时被解析成 `localhost/yggdrasil-runner-pythonatest`（`latest` 拼进名字）。镜像是好的（架构正确），用 `docker tag` 把乱名改回正确名即可。用 `docker image inspect <name> --format '{{.Architecture}}'` 确认是 `amd64` 且 `.Manifests` 为空（单平台，非 manifest list）。

### 构建验证

```bash
# 全部 6 个镜像，架构必须是 amd64
for img in yggdrasil yggdrasil-runner-base yggdrasil-runner-python yggdrasil-runner-node yggdrasil-runner-go yggdrasil-runner-rust; do
  docker image inspect localhost/$img:latest --format "$img: {{.Architecture}} manifests={{.Manifests}}"
done
```

期望：每行 `amd64` 且 `manifests=[]`。

## 导出与传输

```bash
# 导出（主应用 + runners 分两个 tar）
docker save localhost/yggdrasil:latest -o /tmp/yggdrasil-app.tar
docker save \
  localhost/yggdrasil-runner-base:latest \
  localhost/yggdrasil-runner-python:latest \
  localhost/yggdrasil-runner-node:latest \
  localhost/yggdrasil-runner-go:latest \
  localhost/yggdrasil-runner-rust:latest \
  -o /tmp/yggdrasil-runners.tar
gzip -f /tmp/yggdrasil-app.tar /tmp/yggdrasil-runners.tar

# 传输
scp /tmp/yggdrasil-app.tar.gz     xun:/root/docker/yggdrasil/
scp /tmp/yggdrasil-runners.tar.gz xun:/root/docker/yggdrasil/
```

## 服务器导入 + runner 去 `localhost/` 前缀

```bash
ssh xun 'cd /root/docker/yggdrasil && gunzip -kf yggdrasil-app.tar.gz && gunzip -kf yggdrasil-runners.tar.gz'
ssh xun 'docker load -i /root/docker/yggdrasil/yggdrasil-app.tar'
ssh xun 'docker load -i /root/docker/yggdrasil/yggdrasil-runners.tar'
```

**关键：runner 去 `localhost/` 前缀。** `src/api/code_runner/languages.rs` 的 `LANGUAGES` 注册表硬编码镜像名 `yggdrasil-runner-python:latest`（无前缀、无 env 覆盖）。podman 对无前缀名做回退解析，所以必须额外 tag 一份无前缀名。**逐条 ssh 执行（fish 不认 bash for 循环）：**

```bash
ssh xun 'docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-python:latest yggdrasil-runner-python:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-node:latest yggdrasil-runner-node:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-go:latest yggdrasil-runner-go:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-rust:latest yggdrasil-runner-rust:latest'
```

验证（`podman images --format` 显示规范化的完整名，但短名可解析）：

```bash
ssh xun 'podman inspect yggdrasil-runner-python:latest --format "found {{.Architecture}}"'
# 期望: found amd64
```

## compose 文件

`/root/docker/yggdrasil/docker-compose.yml`：

```yaml
services:
  postgres:
    image: docker.io/library/postgres:16-alpine
    container_name: yggdrasil-postgres
    restart: always
    expose: ["5432"]
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER} -d ${POSTGRES_DB}"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    networks: [backend]

  app:
    image: localhost/yggdrasil:latest
    container_name: yggdrasil-app
    restart: always
    expose: ["3000"]
    environment:
      DATABASE_URL: postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
      APP_BASE_URL: https://${VIRTUAL_HOST}
      COOKIE_SECURE: "true"
      TRUSTED_PROXY_COUNT: "1"
      RUST_LOG: info
      DOCKER_SOCKET_PATH: /var/run/docker.sock
      CODE_RUNNER_ALLOW_NETWORK: "false"
      CODE_RUNNER_MAX_CONCURRENT: "2"
      CODE_RUNNER_MAX_CPU_CORES: "1.0"
      CODE_RUNNER_MAX_MEMORY_MB: "512"
      VIRTUAL_HOST: ${VIRTUAL_HOST}
      VIRTUAL_PORT: ${VIRTUAL_PORT}
      LETSENCRYPT_HOST: ${LETSENCRYPT_HOST}
      LETSENCRYPT_EMAIL: ${LETSENCRYPT_EMAIL}
    volumes:
      - uploads_data:/app/uploads
      - backups_data:/app/backups
      - /run/podman/podman.sock:/var/run/docker.sock
    depends_on:
      postgres:
        condition: service_healthy
    networks: [backend, proxy]

volumes:
  postgres_data:
    name: yggdrasil_postgres_data
  uploads_data:
    name: yggdrasil_uploads_data
  backups_data:
    name: yggdrasil_backups_data

networks:
  backend:
    name: yggdrasil_network
  proxy:
    name: nginx-proxy
    external: true
```

`/root/docker/yggdrasil/.env`（密码随机生成，不提交）：

```bash
# 本地生成后 scp 上去
PG_PWD=$(openssl rand -hex 16)
cat > /tmp/.env <<EOF
POSTGRES_USER=yggdrasil
POSTGRES_PASSWORD=$PG_PWD
POSTGRES_DB=yggdrasil
VIRTUAL_HOST=rua.plus
VIRTUAL_PORT=3000
LETSENCRYPT_HOST=rua.plus
LETSENCRYPT_EMAIL=defect.y@qq.com
EOF
scp /tmp/.env xun:/root/docker/yggdrasil/
```

### 关键设计点

| 配置 | 为什么 |
|---|---|
| app `expose` 不 `ports` | 端口由 nginx-proxy 统一接管 80/443，app 不直接暴露 |
| `proxy` 网络 `external: true` | 复用 mimo-blog 同款 nginx-proxy 网络 |
| `VIRTUAL_HOST` 环境变量 | docker-gen 自动生成 vhost + 触发 acme 签证 |
| `/run/podman/podman.sock:/var/run/docker.sock` | podman socket 映射到 app 期望的 docker.sock 路径 |
| `DOCKER_SOCKET_PATH=/var/run/docker.sock` | 告诉 bollard 连容器内的该路径（实际是 podman socket） |
| `CODE_RUNNER_MAX_CONCURRENT=2` | 3.6G 内存服务器，收紧并发避免沙箱压垮宿主 |
| 独立 Postgres 容器 + 独立卷 | 与 mimo-blog 的 blog-postgres 完全隔离 |

## 启动

```bash
ssh xun 'cd /root/docker/yggdrasil && docker compose --env-file .env config --quiet'  # 先验证语法
ssh xun 'cd /root/docker/yggdrasil && docker compose --env-file .env up -d'
```

app 启动时**自动跑 14 个数据库迁移**（`main.rs` 启动钩子，`MIGRATE_STARTUP_TIMEOUT_SECS=30`），无需手动 migrate。Postgres 角色需 CREATEDB（`ensure_database_exists` 会自动建库）。

## 验证（全部必须通过）

### 容器内健康（scratch 镜像没 wget/curl，从 nginx-proxy 容器测）

```bash
ssh xun 'docker exec nginx-proxy curl -s -w "\nHTTP %{http_code}\n" http://yggdrasil-app:3000/healthz'
# 期望: {"status":"ok"}  HTTP 200
ssh xun 'docker exec nginx-proxy curl -s http://yggdrasil-app:3000/readyz'
# 期望: {"db":"ok","pool":{...},"status":"ready"}
```

> **scratch 镜像陷阱**：`docker exec yggdrasil-app <cmd>` 会报 `executable file 'wget' not found`——scratch 运行时没有任何 shell/工具。健康检查必须从**另一个有 curl 的容器**（如 nginx-proxy）发起，或从宿主通过容器 IP。

### 迁移日志

```bash
ssh xun 'docker logs yggdrasil-app 2>&1 | grep -iE "migrat|applied|listen|error|panic"'
# 期望: applying migration 001..014, "successfully applied 14 migration(s)", 无 error/panic
```

### 外部 HTTPS（从本地 curl）

```bash
curl -s https://rua.plus/healthz                      # {"status":"ok"}
curl -sI https://rua.plus/ | grep -iE "HTTP/|strict-transport"  # HTTP/2 200, HSTS
curl -sI http://rua.plus/healthz | grep -i location   # 301 -> https
echo | openssl s_client -connect rua.plus:443 -servername rua.plus 2>/dev/null \
  | openssl x509 -noout -issuer -dates                # issuer=Let's Encrypt
```

### 首位 admin 注册

浏览器访问 `https://rua.plus/register`，**首个注册用户自动成为 admin**（Registration: first user becomes admin，之后注册被拒）。

## Code Runner 限制（已知）

Code Runner 的 5 个沙箱镜像和 API 已部署就绪，但 **`src/infra/docker.rs` 的 tmpfs 选项 `uid=1000,gid=1000` 是 docker 扩展，podman 报 `unknown mount option "uid=1000"`**，实际执行代码会失败。

- 验证镜像本身可跑：`echo 'print("hi")' | ssh xun 'docker run --rm -i --user 1000:1000 --workdir /code yggdrasil-runner-python:latest sh -c "cat > /code/main.py && python -u /code/main.py"'`
- 修复方向：改 `docker.rs` 的 tmpfs 策略（如镜像内预设 `/code` 权限，或用 podman 兼容挂载），修后需重新构建主应用镜像并重新部署

## 清理

```bash
ssh xun 'rm -f /root/docker/yggdrasil/yggdrasil-app.tar /root/docker/yggdrasil/yggdrasil-runners.tar*'
rm -f /tmp/yggdrasil-*.tar* /tmp/.env
```

## 检查清单

完成前确认：

- [ ] 本地 6 个镜像均 `amd64`、`manifests=[]`
- [ ] 服务器 runner 无前缀名可解析（`podman inspect yggdrasil-runner-python:latest` 成功）
- [ ] `docker compose up -d` 两个容器都 Up
- [ ] 迁移日志出现 `successfully applied 14 migration(s)`
- [ ] `healthz` 返回 `{"status":"ok"}`
- [ ] `readyz` 返回 `{"db":"ok",...}`
- [ ] `https://rua.plus/` HTTP/2 200
- [ ] 证书 issuer 是 Let's Encrypt
- [ ] 服务器 tar 已清理

## 更新流程（已部署过，只更新镜像）

```bash
# 1. 本地重新构建主应用（有缓存，快）
docker buildx build --platform linux/amd64 --load -t localhost/yggdrasil:latest .
# 2. 导出传输
docker save localhost/yggdrasil:latest | gzip > /tmp/yggdrasil-app.tar.gz
scp /tmp/yggdrasil-app.tar.gz xun:/root/docker/yggdrasil/
# 3. 服务器导入 + 滚动重启
ssh xun 'cd /root/docker/yggdrasil && gunzip -kf yggdrasil-app.tar.gz && docker load -i yggdrasil-app.tar'
ssh xun 'cd /root/docker/yggdrasil && docker compose --env-file .env up -d app'
# 4. 清理 + 验证
ssh xun 'rm -f /root/docker/yggdrasil/yggdrasil-app.tar*'
curl -s https://rua.plus/healthz
```
