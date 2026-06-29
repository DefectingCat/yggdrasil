# 生产部署指南

> **前置警告：不要直接把 3000 端口暴露到公网 HTTP。**
>
> 本服务端不处理 TLS（这是有意的——应交给反向代理）。但安全相关逻辑
> （会话 Cookie 的 `Secure` 标志、限流按真实客户端 IP 聚合、写请求的 CSRF
> origin 校验）都假设你的部署**前置了一层反向代理并终结 TLS**。
>
> 如果你用 `docker run -p 3000:3000` 把端口直接映射到公网 HTTP，会**同时**踩中
> 三个坑：会话 Cookie 无 `Secure`（明文 HTTP 下可被嗅探）、限流退化或可被伪造、
> CSRF 的 Host 头回退可被构造请求绕过。继续阅读下面的「生产环境必设变量」。

---

## 架构假设

```
 客户端 ──HTTPS──▶ 反向代理(nginx/Caddy) ──HTTP──▶ 应用 :3000
```

反向代理负责：

1. **TLS 终结**——对外只暴露 HTTPS。
2. **注入真实客户端信息**——`X-Forwarded-For`、`X-Real-IP`、`X-Forwarded-Proto`。
3. **清洗客户端传入的转发头**——见下方「⚠️ 反代必须覆写 XFF」。

应用进程本身只监听 `0.0.0.0:3000`（见 `Dockerfile` 的 `IP` / `PORT`），不监听 443，
也不读取任何证书文件。

---

## 生产环境必设变量

在 `.env`（或容器的环境变量）中设置以下三项。`Dockerfile` 与 `.env.example` 默认
值都面向本地开发，**生产环境必须覆盖**：

| 变量 | 生产值 | 作用 |
|------|--------|------|
| `APP_BASE_URL` | `https://your-domain.example` | 写请求 CSRF 校验的可信 origin。不设时回退到请求 `Host` 头 + `X-Forwarded-Proto`，反代后若 `Host` 头可被客户端影响，该回退路径可被 CSRF 绕过。 |
| `COOKIE_SECURE` | `true` | 给会话 Cookie 加 `Secure` 标志，浏览器仅在 HTTPS 下发送。明文 HTTP 生产环境**必开**。 |
| `TRUSTED_PROXY_COUNT` | `1`（单层反代） | 应用前方的反向代理层数，用于从 `X-Forwarded-For` 提取真实客户端 IP。直接对外服务时为 `0`；一层 nginx/Caddy 时为 `1`。 |

### `TRUSTED_PROXY_COUNT` 设错的两种后果

这个值必须**精确等于**实际的反向代理层数：

- **设得比实际大** → 应用会信任客户端伪造的 `X-Forwarded-For`，限流可被任意绕过
  （攻击者每次换一个伪造 IP，每个 IP 独享一个限流桶）。
- **设得比实际小** → 取到的是中间代理的 IP 而非真实客户端 IP，所有真实用户共享
  同一个代理 IP 的限流桶，正常用户互相挤占。

---

## 反向代理配置示例

### nginx

```nginx
# /etc/nginx/conf.d/yggdrasil.conf

# 1. HTTP 强制跳转 HTTPS
server {
    listen 80;
    server_name your-domain.example;
    return 301 https://$host$request_uri;
}

# 2. HTTPS 主服务
server {
    listen 443 ssl;
    http2 on;
    server_name your-domain.example;

    ssl_certificate     /etc/letsencrypt/live/your-domain.example/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.example/privkey.pem;

    # ⚠️ 关键：上传路由的 body 上限。应用侧硬限制 10 MiB（见 src/main.rs 的
    # DefaultBodyLimit::max(10 * 1024 * 1024)），nginx 默认仅 1 MiB 会先于应用
    # 返回 413。设成略大于应用上限即可。
    client_max_body_size 12m;

    location / {
        proxy_pass http://127.0.0.1:3000;

        # ⚠️ 反代必须覆写 XFF——见下方「⚠️ 反代必须覆写 XFF」说明。
        # proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Host $host;

        # 上传与图片处理可能较慢（图片转码最长 300s 超时），反代代理超时
        # 要大于应用侧超时，否则反代先于应用返回 504。
        proxy_read_timeout 360s;
        proxy_send_timeout 360s;
    }

    # 健康检查：可不经 TLS 或单独配置探针路径。
    # GET /healthz —— liveness，进程存活即 200，不查 DB。
    # GET /readyz  —— readiness，执行 SELECT 1 检测 DB 连通性，不可达返回 503。
}
```

### Caddy

Caddy 会自动申请并续期 Let's Encrypt 证书，配置更短：

```caddy
your-domain.example {
    # ⚠️ 上传 body 上限，理由同 nginx。应用侧硬限制 10 MiB。
    request_body {
        max_size 12MB
    }

    reverse_proxy 127.0.0.1:3000 {
        # Caddy 默认会设置 X-Forwarded-For / X-Forwarded-Proto / Host，
        # 并正确处理 XFF 的追加（详见下方说明）。

        # 上传/图片处理超时对齐应用侧（300s）。
        transport http {
            read_timeout 360s
            write_timeout 360s
        }
    }
}
```

### ⚠️ 反代必须覆写 `X-Forwarded-For`

应用的 `TRUSTED_PROXY_COUNT=1` 假设：`X-Forwarded-For` 的**最右一项**是可信的反代
自己写入的，其余左侧项才可能是客户端原始 IP。因此反代必须**覆写或正确追加** XFF：

- **nginx**：`proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;`
  ——`$proxy_add_x_forwarded_for` 会把客户端**已发送的** XFF 与反代看到的
  `$remote_addr` 拼接。由于应用只信任右侧第 1 项（反代写入的 `$remote_addr`），
  客户端伪造的左侧项会被正确忽略。
- **Caddy**：`reverse_proxy` 默认行为已满足，无需手动配置。

**绝不能**做的是把应用直接暴露到公网、或让客户端的 `X-Forwarded-For` 原封不动地
成为最右一项——那等于把限流的 key 交给攻击者随意伪造。

---

## 完整环境变量清单

除上述三项必设变量外，其余变量均有合理默认值，按需调整。完整说明见
[`.env.example`](../.env.example)。生产部署最少需要：

```bash
# 数据库（必填）
DATABASE_URL=postgres://user:password@db-host:5432/yggdrasil

# 日志
RUST_LOG=info

# === 生产安全三件套（必设）===
APP_BASE_URL=https://your-domain.example
COOKIE_SECURE=true
TRUSTED_PROXY_COUNT=1
```

其余（限流阈值、WebP 编码参数、图片缓存上限、连接池大小等）均为可选调优项，
不设时使用 `.env.example` 中的默认值。

---

## Docker 部署

```bash
docker build -t yggdrasil .

docker run -d \
  --name yggdrasil \
  -p 127.0.0.1:3000:3000 \
  -e DATABASE_URL=postgres://user:password@db-host:5432/yggdrasil \
  -e APP_BASE_URL=https://your-domain.example \
  -e COOKIE_SECURE=true \
  -e TRUSTED_PROXY_COUNT=1 \
  -e RUST_LOG=info \
  -v yggdrasil-uploads:/app/uploads \
  yggdrasil
```

注意 `-p 127.0.0.1:3000:3000`——只绑定到回环地址，让反向代理作为唯一的公网入口。
**不要**用 `-p 3000:3000`（绑定 `0.0.0.0`），那会让应用绕过反代直接暴露。

数据库迁移需在容器首次启动前执行：在宿主机用 `./migrate.sh`（需本地 PostgreSQL
客户端工具），或在初始化容器中运行。

---

## 部署后验证清单

部署完成后，逐项确认：

- [ ] `curl -I https://your-domain.example/healthz` 返回 `200`。
- [ ] `curl -I https://your-domain.example/readyz` 返回 `200`（DB 连通）。
- [ ] 浏览器登录后，DevTools → Application → Cookies 中 `session` 项的
      `Secure` 列为勾选、`HttpOnly` 为勾选、`SameSite` 为 `Lax`。
- [ ] HTTP 访问（`http://your-domain.example`）被 301 跳转到 HTTPS。
- [ ] 反代访问日志中记录的是真实客户端公网 IP，而非反代自身的回环地址
      （确认 `TRUSTED_PROXY_COUNT` 配置正确，限流按真实 IP 生效）。
