#!/bin/sh
# 构建代码运行器所需的沙箱镜像：base → python / node。
#
# 依赖：本机已安装 docker。构建顺序固定（python/node FROM base）。
# 镜像 tag 与 src/api/code_runner/languages.rs 的注册项严格对应：
#   yggdrasil-runner-base:latest
#   yggdrasil-runner-python:latest
#   yggdrasil-runner-node:latest
set -e

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

echo "==> Building yggdrasil-runner-base:latest"
docker build -t yggdrasil-runner-base:latest "$SCRIPT_DIR/runner-base"

echo "==> Building yggdrasil-runner-python:latest"
docker build -t yggdrasil-runner-python:latest "$SCRIPT_DIR/runner-python"

echo "==> Building yggdrasil-runner-node:latest"
docker build -t yggdrasil-runner-node:latest "$SCRIPT_DIR/runner-node"

echo "==> Done. Images:"
docker images --filter "reference=yggdrasil-runner-*" \
    --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"
