#!/bin/sh
# 构建代码运行器所需的沙箱镜像：base → python / node / go / rust。
#
# 依赖：本机已安装 docker。构建顺序固定（python/node/go/rust 均为 FROM base）。
# 镜像 tag 与 src/api/code_runner/languages.rs 的注册项严格对应：
#   yggdrasil-runner-base:latest
#   yggdrasil-runner-python:latest
#   yggdrasil-runner-node:latest
#   yggdrasil-runner-go:latest
#   yggdrasil-runner-rust:latest
set -e

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

echo "==> Building yggdrasil-runner-base:latest"
docker build -t yggdrasil-runner-base:latest "$SCRIPT_DIR/runner-base"

echo "==> Building yggdrasil-runner-python:latest"
docker build -t yggdrasil-runner-python:latest "$SCRIPT_DIR/runner-python"

echo "==> Building yggdrasil-runner-node:latest"
docker build -t yggdrasil-runner-node:latest "$SCRIPT_DIR/runner-node"

echo "==> Building yggdrasil-runner-go:latest"
docker build -t yggdrasil-runner-go:latest "$SCRIPT_DIR/runner-go"

echo "==> Building yggdrasil-runner-rust:latest"
docker build -t yggdrasil-runner-rust:latest "$SCRIPT_DIR/runner-rust"

echo "==> Done. Images:"
docker images --filter "reference=yggdrasil-runner-*" \
    --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"
