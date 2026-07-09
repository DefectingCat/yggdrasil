# Go 运行沙箱镜像。
# 与 src/api/code_runner/languages.rs 的 go 注册项对齐：
#   image: yggdrasil-runner-go:latest
#   run_cmd: go run /code/main.go
#
# go run 是单条命令（内部编译 + 运行），可直接作为 run_cmd（无需 wrapper）。
# 注意：运行时只读根文件系统 + tmpfs /code /tmp，$HOME/.cache 不可写，
# 必须把 GOCACHE / GOTMPDIR / GOPATH 重定向到可写的 /tmp tmpfs。
FROM yggdrasil-runner-base:latest

RUN apk add --no-cache go \
    && mkdir -p /tmp/go-cache /tmp/gopath \
    && chown -R runner:runner /tmp/go-cache /tmp/gopath

ENV GOCACHE=/tmp/go-cache \
    GOTMPDIR=/tmp \
    GOPATH=/tmp/gopath

COPY runner.toml /runner.toml

USER runner
