---
name: deploy-to-linux
description: |
  将 yggdrasil 部署/更新到任意 Linux 服务器时使用。本地 arm64 用 Docker Rosetta 构建
  x86 镜像，导出传输到目标服务器（Docker 或 Podman）后用 docker compose 运行，
  前置反代自动签发 HTTPS。触发关键词："部署"、"deploy"、"发布到服务器"、"上线"、"docker compose"。
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - AskUserQuestion
  - Agent
metadata:
  trigger: 部署到 Linux 服务器、Docker 构建 x86 镜像、docker compose 上线、更新线上
---

# Deploy to Linux: 部署/更新 yggdrasil 到任意 Linux 服务器

从本地 arm64 构建器构建 `linux/amd64` 镜像，传输到目标 Linux 服务器并以 docker compose 运行。前置反代负责 HTTPS。

**这是项目专属 skill**：镜像名、`LANGUAGES` 注册表硬编码、runner 沙箱、nginx-proxy 约定都绑定到 yggdrasil 本身，但服务器探测、构建、传输、compose 编排是通用流程。

## 第 0 步：探测目标服务器（每次部署必做）

服务器环境未知，必须先探测再决策。一条命令拿全关键信息：

```bash
ssh <host> 'echo "OS:"; uname -sm; echo "容器运行时:"; docker --version 2>&1 || echo "NO_DOCKER"; echo "compose:"; docker compose version 2>&1 | head -1; echo "默认shell:"; echo $SHELL; echo "socket:"; ls -la /var/run/docker.sock /run/podman/podman.sock 2>&1; echo "磁盘:"; df -h / | tail -1; echo "内存:"; free -h 2>/dev/null | grep Mem; echo "端口占用:"; ss -tlnp 2>/dev/null | grep -E ":80 |:443 |:3000 " | head; echo "已有容器:"; docker ps --format "{{.Names}}" 2>&1 | head'
```

根据输出填这张决策表：

| 探测项 | 可能值 | 决策 |
|---|---|---|
| `docker --version` | `Docker version ...` | 真 Docker，socket 在 `/var/run/docker.sock` |
| | `podman version ...` | **Podman**（`docker` 是别名），socket 在 `/run/podman/podman.sock` |
| | `NO_DOCKER` | 需先装 Docker 或 Podman（本文不覆盖） |
| `$SHELL` | `/bin/bash` 或 `/bin/sh` | 可在 ssh 命令里写 bash 逻辑 |
| | `/usr/bin/fish` | **fish 陷阱**，见下，避免在 ssh 里写 shell 逻辑 |
| 端口 80/443 占用 | 已被占用 | **复用现有反代**（探测它是不是 nginx-proxy） |
| | 空闲 | **自建反代**（nginx 容器或复用 nginx-proxy） |

## fish shell 陷阱（若服务器是 fish）

fish 不是 bash，ssh 进去的命令在 fish 里解析会撞上：

| bash 语法 | fish 报错 | 解决 |
|---|---|---|
| `VAR=$(cmd)` | `Unsupported use of '='` | 避免服务器端赋值，或用 `set VAR (cmd)` |
| `for x in a b; do ...; done` | `Missing end to balance this for loop` | 用 `for x in a b; ...; end` |
| `echo $?` | `$? is not the exit status` | 用 `$status` |
| `&&` 链带赋值 | 中途失败静默 | **拆成多条独立 ssh，每条一条命令** |

**最稳的做法（适用于任何 shell）：避免在 ssh 命令里写 shell 逻辑。** 每条 `ssh <host> 'cmd'` 只跑一条命令，需要循环时在本地 bash 里循环、循环体内逐条 ssh。

## Docker vs Podman 关键差异

| 项 | Docker | Podman |
|---|---|---|
| socket | `/var/run/docker.sock` | `/run/podman/podman.sock` |
| 运行模式 | 通常 rootful daemon | rootful 或 rootless（探测 `podman info` 的 `rootless`） |
| 镜像短名 | `yggdrasil-runner-python` 直接可用 | 短名回退解析，但 `podman images` 显示规范化完整名 |
| tmpfs `uid=`/`gid=` 选项 | 支持 | **不支持**（报 `unknown mount option`）→ 影响 Code Runner，见末尾 |
| `docker-compose.yml` | 完全兼容 | 完全兼容（podman-compose 或 docker-compose v2 provider） |

**compose 里挂 socket 时按实际路径映射**：
- Docker: `- /var/run/docker.sock:/var/run/docker.sock`
- Podman: `- /run/podman/podman.sock:/var/run/docker.sock`（app 代码读 `DOCKER_SOCKET_PATH`，映射到容器内统一路径）

## 本地构建（arm64 Mac → linux/amd64）

### 前提：开启 Docker Desktop Rosetta

arm64 Mac 上用 QEMU 模拟 amd64 跑 `rustc` 会 **SIGSEGV**（`qemu: uncaught target signal 11`），必须用 Rosetta 转译：

```bash
grep UseVirtualizationFrameworkRosetta ~/Library/Group\ Containers/group.com.docker/settings-store.json
# 期望: "UseVirtualizationFrameworkRosetta": true
```

若为 false：Docker Desktop 设置里勾选 "Use Virtualization framework" + "Use Rosetta for x86/amd64"，重启 Docker。若服务器是 arm64 则跳过本节、用原生 `docker build`。

### 主应用镜像

```bash
docker buildx build --platform linux/amd64 --load -t localhost/yggdrasil:latest .
```

- Dockerfile 用 `dpkg --print-architecture` 检测架构，amd64 腿原生构建 `x86_64-unknown-linux-musl` 静态二进制
- 首次约 15-30 分钟（Rosetta 下 cargo 全量编译），有 buildkit 缓存后分钟级
- 产物 `localhost/yggdrasil:latest`，scratch 运行时层约 16MB

### 5 个 Code Runner 沙箱镜像

runner Dockerfile `FROM yggdrasil-runner-base:latest`（无 `localhost/` 前缀），必须**先建 base 再建子镜像**：

```bash
# 1. base 先用 localhost/ 前缀建
docker buildx build --platform linux/amd64 --load \
  -t localhost/yggdrasil-runner-base:latest docker/runner-base
# 2. 再 tag 无前缀名,让子镜像 FROM 能解析
docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest
# 3. 4 个子镜像(它们 FROM yggdrasil-runner-base:latest)
for img in python node go rust; do
  docker buildx build --platform linux/amd64 --load \
    -t localhost/yggdrasil-runner-$img:latest docker/runner-$img
  docker tag localhost/yggdrasil-runner-$img:latest yggdrasil-runner-$img:latest
done
```

> **buildx v0.35 命名 bug**：`-t localhost/yggdrasil-runner-python:latest` 有时被解析成 `localhost/yggdrasil-runner-pythonatest`（`latest` 拼进名字）。镜像是好的（架构正确），用 `docker tag` 把乱名改回正确名即可。

### 构建验证

```bash
for img in yggdrasil yggdrasil-runner-base yggdrasil-runner-python yggdrasil-runner-node yggdrasil-runner-go yggdrasil-runner-rust; do
  docker image inspect localhost/$img:latest --format "$img: {{.Architecture}} manifests={{.Manifests}}"
done
# 期望: 每行 amd64 且 manifests=[](单平台,非 manifest list)
```

## 导出与传输

```bash
# 导出(主应用 + runners 分两个 tar)
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
scp /tmp/yggdrasil-app.tar.gz     <host>:/root/docker/yggdrasil/
scp /tmp/yggdrasil-runners.tar.gz <host>:/root/docker/yggdrasil/
```

部署目录用 `<host>:/root/docker/yggdrasil/`（root 用户约定；非 root 换 `~/docker/yggdrasil/`）。

## 服务器导入 + runner 去 `localhost/` 前缀

```bash
ssh <host> 'mkdir -p /root/docker/yggdrasil'
ssh <host> 'cd /root/docker/yggdrasil && gunzip -kf yggdrasil-app.tar.gz && gunzip -kf yggdrasil-runners.tar.gz'
ssh <host> 'docker load -i /root/docker/yggdrasil/yggdrasil-app.tar'
ssh <host> 'docker load -i /root/docker/yggdrasil/yggdrasil-runners.tar'
```

**关键：runner 去 `localhost/` 前缀。** `src/api/code_runner/languages.rs` 的 `LANGUAGES` 注册表硬编码镜像名 `yggdrasil-runner-python:latest`（无前缀、无 env 覆盖）。必须额外 tag 一份无前缀名。**逐条 ssh 执行（fish 不认 bash for 循环）：**

```bash
ssh <host> 'docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest'
ssh <host> 'docker tag localhost/yggdrasil-runner-python:latest yggdrasil-runner-python:latest'
ssh <host> 'docker tag localhost/yggdrasil-runner-node:latest yggdrasil-runner-node:latest'
ssh <host> 'docker tag localhost/yggdrasil-runner-go:latest yggdrasil-runner-go:latest'
ssh <host> 'docker tag localhost/yggdrasil-runner-rust:latest yggdrasil-runner-rust:latest'
```

验证短名可解析：

```bash
ssh <host> 'docker inspect yggdrasil-runner-python:latest --format "found {{.Architecture}}"'
# 期望: found amd64
```

## compose 编排

### `.env`（随机密码，不提交）

```bash
PG_PWD=$(openssl rand -hex 16)
cat > /tmp/.env <<EOF
POSTGRES_USER=yggdrasil
POSTGRES_PASSWORD=$PG_PWD
POSTGRES_DB=yggdrasil
# 反代与证书(三选一,见下"前置反代"决策)
VIRTUAL_HOST=<你的域名>
VIRTUAL_PORT=3000
LETSENCRYPT_HOST=<你的域名>
LETSENCRYPT_EMAIL=<你的邮箱>
EOF
scp /tmp/.env <host>:/root/docker/yggdrasil/
```

### `docker-compose.yml`（通用模板）

以下模板**默认假设复用 nginx-proxy**（最常见情况）。socket 挂载路径按探测结果二选一（见注释）：

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
    expose: ["3000"]              # 不 ports 映射,由反代接管
    environment:
      DATABASE_URL: postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
      APP_BASE_URL: https://${VIRTUAL_HOST}
      COOKIE_SECURE: "true"
      TRUSTED_PROXY_COUNT: "1"    # 反代层数,见下
      RUST_LOG: info
      DOCKER_SOCKET_PATH: /var/run/docker.sock
      CODE_RUNNER_ALLOW_NETWORK: "false"
      CODE_RUNNER_MAX_CONCURRENT: "2"
      CODE_RUNNER_MAX_CPU_CORES: "1.0"
      CODE_RUNNER_MAX_MEMORY_MB: "512"
      # nginx-proxy 自动发现(复用模式才需要)
      VIRTUAL_HOST: ${VIRTUAL_HOST}
      VIRTUAL_PORT: ${VIRTUAL_PORT}
      LETSENCRYPT_HOST: ${LETSENCRYPT_HOST}
      LETSENCRYPT_EMAIL: ${LETSENCRYPT_EMAIL}
    volumes:
      - uploads_data:/app/uploads
      - backups_data:/app/backups
      # Docker: /var/run/docker.sock:/var/run/docker.sock
      # Podman: /run/podman/podman.sock:/var/run/docker.sock
      - /run/podman/podman.sock:/var/run/docker.sock
    depends_on:
      postgres:
        condition: service_healthy
    networks: [backend, proxy]

volumes:
  postgres_data:
  uploads_data:
  backups_data:

networks:
  backend:
    name: yggdrasil_network
  proxy:
    # 复用现有 nginx-proxy: external: true + name: nginx-proxy
    # 自建反代: 去掉 external,在这里定义一个普通网络,反代容器也加入它
    name: nginx-proxy
    external: true
```

### 关键设计点

| 配置 | 为什么 |
|---|---|
| app `expose` 不 `ports` | 端口由反代统一接管，app 不直接暴露 |
| `VIRTUAL_HOST` 环境变量 | nginx-proxy 的 docker-gen 自动生成 vhost + 触发 acme 签证 |
| socket 映射到容器内 `/var/run/docker.sock` | app 代码 `DOCKER_SOCKET_PATH` 默认该路径，映射后 podman/docker 都通 |
| `CODE_RUNNER_MAX_CONCURRENT=2` | 小内存服务器收紧并发避免沙箱压垮宿主，按服务器内存调整 |
| 独立 Postgres 容器 + 独立卷 | 与服务器上其他应用的 DB 隔离 |
| `TRUSTED_PROXY_COUNT` | 反代层数，决定 X-Forwarded-For 取第几个 IP。一层反代填 1 |

## 前置反代（三选一，按探测结果决策）

### 方案 A：复用现有 nginx-proxy（推荐，端口 80/443 已被占时）

前提：探测到服务器已跑 `nginxproxy/nginx-proxy` + `acme-companion`（占 80/443）。

- compose 的 `proxy` 网络设 `external: true` + `name: nginx-proxy`
- app 设 `VIRTUAL_HOST`/`VIRTUAL_PORT`/`LETSENCRYPT_HOST`/`LETSENCRYPT_EMAIL` 环境变量
- docker-gen 自动发现容器、生成 vhost、触发 acme 签证
- 无需额外组件，与现有应用共享反代

### 方案 B：自建 nginx-proxy（80/443 空闲，想用自动签证）

```yaml
# 在同一个 compose 里加两个服务(或独立 compose)
  nginx-proxy:
    image: nginxproxy/nginx-proxy:1.6
    ports: ["80:80", "443:443"]
    volumes:
      - /var/run/docker.sock:/tmp/docker.sock:ro   # podman 换 /run/podman/podman.sock
      - ./certs:/etc/nginx/certs
      - ./vhost.d:/etc/nginx/vhost.d
      - ./html:/usr/share/nginx/html
    networks: [proxy]
  acme:
    image: nginxproxy/acme-companion
    depends_on: [nginx-proxy]
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./certs:/etc/nginx/certs
      - ./vhost.d:/etc/nginx/vhost.d
      - ./html:/usr/share/nginx/html
    environment:
      NGINX_PROXY_CONTAINER: nginx-proxy
    networks: [proxy]
```

app 的 `proxy` 网络去掉 `external: true`，改为普通网络，nginx-proxy 也加入。

### 方案 C：自建 nginx + 手动证书 / Cloudflare 代理

80/443 空闲但不想用 nginx-proxy：起一个普通 nginx 容器，手动配 `server` 块 `proxy_pass http://yggdrasil-app:3000;`，证书用 certbot 或 Cloudflare Origin 证书。compose 里加：

```yaml
  nginx:
    image: nginx:alpine
    ports: ["80:80", "443:443"]
    volumes:
      - ./nginx.conf:/etc/nginx/conf.d/default.conf:ro
      - ./certs:/etc/nginx/certs:ro
    networks: [proxy]
```

app 的 `proxy` 网络去掉 `external: true`，nginx 也加入。`nginx.conf` 关键：`client_max_body_size 12m;`（匹配 10MB 上传限制）、`proxy_read_timeout 360s;`、转发 `X-Forwarded-For`/`X-Forwarded-Proto`。

## 启动与验证

```bash
ssh <host> 'cd /root/docker/yggdrasil && docker compose --env-file .env config --quiet'  # 验证语法
ssh <host> 'cd /root/docker/yggdrasil && docker compose --env-file .env up -d'
```

app 启动时**自动跑数据库迁移**（`main.rs` 启动钩子），无需手动 migrate。Postgres 角色需 CREATEDB（自动建库）。

### 验证清单（全部必须通过）

```bash
# 1. 容器状态
ssh <host> 'docker ps --filter name=yggdrasil --format "{{.Names}} {{.Status}}"'
# 期望: yggdrasil-postgres (healthy) + yggdrasil-app (up)

# 2. 迁移日志(关键!确认迁移成功)
ssh <host> 'docker logs yggdrasil-app 2>&1 | grep -iE "migrat|applied|error|panic"'
# 期望: "successfully applied 14 migration(s)", 无 error/panic

# 3. 健康检查(scratch 镜像没 wget/curl,从反代容器或同网络容器测)
ssh <host> 'docker exec nginx-proxy curl -s http://yggdrasil-app:3000/healthz'
# 期望: {"status":"ok"}
ssh <host> 'docker exec nginx-proxy curl -s http://yggdrasil-app:3000/readyz'
# 期望: {"db":"ok","pool":{...},"status":"ready"}

# 4. 外部 HTTPS(从本地 curl)
curl -s https://<域名>/healthz                          # {"status":"ok"}
curl -sI https://<域名>/ | grep -iE "HTTP/|strict-transport"  # HTTP/2 200 + HSTS
curl -sI http://<域名>/ | grep -i location              # 301 -> https(若启用强制跳转)

# 5. 证书
echo | openssl s_client -connect <域名>:443 -servername <域名> 2>/dev/null \
  | openssl x509 -noout -issuer -dates
```

### 首位 admin 注册

浏览器访问 `https://<域名>/register`，**首个注册用户自动成为 admin**（之后注册被拒）。

### scratch 镜像陷阱

`docker exec yggdrasil-app <cmd>` 会报 `executable file 'wget' not found`——scratch 运行时没有任何 shell/工具。健康检查必须从**另一个有 curl 的容器**（反代或 `nginx-proxy`）发起，或从宿主通过容器 IP/端口映射。

## DNS 前置条件

域名必须先解析到服务器 IP，acme 才能完成 HTTP-01 验证：

```bash
dig +short <域名>            # 应返回服务器公网 IP
ssh <host> 'curl -s ifconfig.me'  # 服务器公网 IP,两者要对上
```

注意 IPv4（A 记录）和 IPv6（AAAA 记录）可能不一致，acme HTTP-01 默认走 IPv4。若 AAAA 指向错误子网，仅 IPv6 客户端受影响。

## Code Runner 限制（已知）

Code Runner 的 5 个沙箱镜像和 API 已部署就绪，但 **`src/infra/docker.rs` 的 tmpfs 选项 `uid=1000,gid=1000` 是 docker 扩展，podman 报 `unknown mount option "uid=1000"`**，实际执行代码会失败。

- Docker 服务器：不受影响，正常工作
- Podman 服务器：Code Runner 执行会失败。镜像本身可跑（验证）：`echo 'print("hi")' | ssh <host> 'docker run --rm -i --user 1000:1000 --workdir /code yggdrasil-runner-python:latest sh -c "cat > /code/main.py && python -u /code/main.py"'`
- 修复方向：改 `docker.rs` 的 tmpfs 策略（镜像内预设 `/code` 权限，或用 podman 兼容挂载），修后需重新构建主应用镜像

## 清理

```bash
ssh <host> 'rm -f /root/docker/yggdrasil/yggdrasil-app.tar* /root/docker/yggdrasil/yggdrasil-runners.tar*'
rm -f /tmp/yggdrasil-*.tar* /tmp/.env
```

## 更新流程（已部署过，只更新镜像）

```bash
# 1. 本地重新构建主应用(有缓存,快)
docker buildx build --platform linux/amd64 --load -t localhost/yggdrasil:latest .
# 2. 导出传输
docker save localhost/yggdrasil:latest | gzip > /tmp/yggdrasil-app.tar.gz
scp /tmp/yggdrasil-app.tar.gz <host>:/root/docker/yggdrasil/
# 3. 服务器导入 + 滚动重启
ssh <host> 'cd /root/docker/yggdrasil && gunzip -kf yggdrasil-app.tar.gz && docker load -i yggdrasil-app.tar'
ssh <host> 'cd /root/docker/yggdrasil && docker compose --env-file .env up -d app'
# 4. 清理 + 验证
ssh <host> 'rm -f /root/docker/yggdrasil/yggdrasil-app.tar*'
curl -s https://<域名>/healthz
```
