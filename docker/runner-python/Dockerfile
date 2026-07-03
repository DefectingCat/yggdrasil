# Python 运行沙箱镜像。
# 与 src/api/code_runner/languages.rs 的 python 注册项对齐：
#   image: yggdrasil-runner-python:latest
#   run_cmd: python /code/main.py
#
# 注意：alpine 的 python3 包只提供 `python3`，这里建 `python` 符号链接以匹配 run_cmd。
# 运行时（src/infra/docker.rs）以 user 1000:1000、只读根文件系统 + tmpfs /code 挂载容器，
# 源码通过 stdin 写入 /code/main.py 后 exec run_cmd。
FROM yggdrasil-runner-base:latest

RUN apk add --no-cache python3 \
    && ln -sf /usr/bin/python3 /usr/bin/python

COPY runner.toml /runner.toml

USER runner
