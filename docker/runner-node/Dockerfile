# Node.js 运行沙箱镜像。
# 与 src/api/code_runner/languages.rs 的 node 注册项对齐：
#   image: yggdrasil-runner-node:latest
#   run_cmd: node /code/main.js
FROM yggdrasil-runner-base:latest

RUN apk add --no-cache nodejs

COPY runner.toml /runner.toml

USER runner
