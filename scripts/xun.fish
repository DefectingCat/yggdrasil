#!/usr/bin/env fish

# Yggdrasil 全量部署到 xun(Podman + fish + nginx-proxy 反代)
# 主应用 + 5 个 Code Runner 沙箱镜像全量重建、传输、导入、滚动重启
# 用法: fish scripts/xun.fish

# 5 个 runner 子镜像(base 由脚本自己先建,FROM 它)
set -l RUNNERS python node go rust bun

echo "==> [1/7] 构建 runner base 镜像"
docker buildx build --platform linux/amd64 --load \
  -t localhost/yggdrasil-runner-base:latest docker/runner-base; or exit 1
docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest; or exit 1

echo "==> [2/7] 构建主应用镜像(透传 git 信息)"
docker buildx build --platform linux/amd64 --load \
  --build-arg YGG_BUILD_GIT_DESCRIBE=(git describe --tags --always --dirty) \
  --build-arg YGG_BUILD_GIT_HASH=(git rev-parse HEAD) \
  --build-arg YGG_BUILD_GIT_COMMIT_DATE=(git log -1 --format=%cd --date=iso-strict) \
  -t localhost/yggdrasil:latest .; or exit 1

echo "==> [3/7] 构建 5 个 runner 子镜像"
for img in $RUNNERS
    echo "  -- $img"
    docker buildx build --platform linux/amd64 --load \
      -t localhost/yggdrasil-runner-$img:latest docker/runner-$img; or exit 1
    docker tag localhost/yggdrasil-runner-$img:latest yggdrasil-runner-$img:latest; or exit 1
end

echo "==> [4/7] 构建验证(期望全 amd64)"
set -l ALL_IMAGES yggdrasil yggdrasil-runner-base
for img in $RUNNERS
    set -a ALL_IMAGES yggdrasil-runner-$img
end
for img in $ALL_IMAGES
    set -l arch (docker image inspect localhost/$img:latest --format "{{.Architecture}}")
    echo "  $img: $arch"
    if test "$arch" != "amd64"
        echo "  架构错误!期望 amd64" >&2
        exit 1
    end
end

echo "==> [5/7] 导出 + 传输到 xun"
# 主应用单独一个 tar(滚动重启时只 reload 它)
docker save localhost/yggdrasil:latest -o /tmp/yggdrasil-app.tar; or exit 1
# 6 个 runner 镜像打包成一个 tar(base + 5 子镜像)
docker save \
  localhost/yggdrasil-runner-base:latest \
  localhost/yggdrasil-runner-python:latest \
  localhost/yggdrasil-runner-node:latest \
  localhost/yggdrasil-runner-go:latest \
  localhost/yggdrasil-runner-rust:latest \
  localhost/yggdrasil-runner-bun:latest \
  -o /tmp/yggdrasil-runners.tar; or exit 1
gzip -f /tmp/yggdrasil-app.tar /tmp/yggdrasil-runners.tar; or exit 1
scp /tmp/yggdrasil-app.tar.gz /tmp/yggdrasil-runners.tar.gz xun:/root/docker/yggdrasil/; or exit 1
rm -f /tmp/yggdrasil-app.tar.gz /tmp/yggdrasil-runners.tar.gz

echo "==> [6/7] 服务器导入 + runner 去前缀 + 滚动重启 app"
ssh xun 'cd /root/docker/yggdrasil && gunzip -kf yggdrasil-app.tar.gz && gunzip -kf yggdrasil-runners.tar.gz'; or exit 1
ssh xun 'docker load -i /root/docker/yggdrasil/yggdrasil-app.tar'; or exit 1
ssh xun 'docker load -i /root/docker/yggdrasil/yggdrasil-runners.tar'; or exit 1
# runner 去 localhost/ 前缀:LANGUAGES 注册表硬编码 yggdrasil-runner-*:latest(无前缀、无 env 覆盖)
# fish 不认 bash for 循环,逐条 ssh 执行
ssh xun 'docker tag localhost/yggdrasil-runner-base:latest yggdrasil-runner-base:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-python:latest yggdrasil-runner-python:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-node:latest yggdrasil-runner-node:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-go:latest yggdrasil-runner-go:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-rust:latest yggdrasil-runner-rust:latest'
ssh xun 'docker tag localhost/yggdrasil-runner-bun:latest yggdrasil-runner-bun:latest'
# 滚动重启:只重建 app 容器(postgres 和数据卷不动)
ssh xun 'cd /root/docker/yggdrasil && docker compose --env-file .env up -d app'; or exit 1
ssh xun 'rm -f /root/docker/yggdrasil/yggdrasil-app.tar* /root/docker/yggdrasil/yggdrasil-runners.tar*'

echo "==> [7/7] 验证"
echo "--- 容器状态(期望 postgres healthy + app up)---"
ssh xun 'docker ps --filter name=yggdrasil --format "{{.Names}} {{.Status}}"'
echo "--- 迁移日志(期望 applied,无 error/panic)---"
ssh xun 'docker logs yggdrasil-app 2>&1 | grep -iE "migrat|error|panic" | tail'
echo "--- runner 镜像(期望全 found amd64)---"
ssh xun 'docker inspect yggdrasil-runner-base:latest --format "base: found {{.Architecture}}"'
ssh xun 'docker inspect yggdrasil-runner-python:latest --format "python: found {{.Architecture}}"'
ssh xun 'docker inspect yggdrasil-runner-node:latest --format "node: found {{.Architecture}}"'
ssh xun 'docker inspect yggdrasil-runner-go:latest --format "go: found {{.Architecture}}"'
ssh xun 'docker inspect yggdrasil-runner-rust:latest --format "rust: found {{.Architecture}}"'
ssh xun 'docker inspect yggdrasil-runner-bun:latest --format "bun: found {{.Architecture}}"'
echo "--- 健康检查 ---"
ssh xun 'docker exec nginx-proxy curl -s http://yggdrasil-app:3000/healthz'; echo
ssh xun 'docker exec nginx-proxy curl -s http://yggdrasil-app:3000/readyz'; echo
echo "--- git 版本头(确认不再是 unknown)---"
ssh xun 'docker exec nginx-proxy curl -sI http://yggdrasil-app:3000/ | grep -i x-yggdrasil-git'
echo "--- 外部 HTTPS ---"
curl -s https://rua.plus/healthz; echo
curl -sI https://rua.plus/ | grep -iE "HTTP/|strict-transport"

echo "==> 部署完成"
