# 代码运行器的共享基础镜像。
# 仅创建非 root 用户 1000:1000 与 /code 工作目录；具体语言镜像在此基础上安装运行时。
# 与 src/infra/docker.rs 的 user: "1000:1000"、working_dir: "/code" 对齐。
FROM alpine:3.18

RUN addgroup -S runner && adduser -S runner -G runner -u 1000 \
    && mkdir -p /code \
    && chown -R runner:runner /code

WORKDIR /code
