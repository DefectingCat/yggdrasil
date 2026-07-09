#!/bin/sh
# Rust 源码编译并运行的 wrapper。
#
# 背景：src/infra/docker.rs 的源码注入脚本为
#   sh -c "cat > /code/main.rs && exec {run_cmd}"
# 其中 exec 会用 run_cmd 进程替换 sh 进程。若直接写
#   "rustc -o /tmp/main /code/main.rs && /tmp/main"
# exec 替换后 && 后半段永远不会执行（exec 成功即不再返回 shell）。
# 因此必须把「编译 + 运行」封装进本脚本，由 sh 解释执行：
#   rustc 编译成功后，再用 exec 替换为编译产物，避免多一层常驻进程。
#
# 编译产物写到 /tmp/main（tmpfs 64m 可写；只读根文件系统下 / 不可写）。
rustc -o /tmp/main /code/main.rs && exec /tmp/main
