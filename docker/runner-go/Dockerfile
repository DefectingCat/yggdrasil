# Go 运行沙箱镜像。
# 与 src/api/code_runner/languages.rs 的 go 注册项对齐：
#   image: yggdrasil-runner-go:latest
#   run_cmd: go run /code/main.go
#   cache_volume: yggdrasil-gocache → /go-cache（src/infra/docker.rs 挂载）
#
# go run 是单条命令（内部编译 + 运行），可直接作为 run_cmd（无需 wrapper）。
# 运行时只读根文件系统 + tmpfs /code /tmp，$HOME/.cache 不可写。
# GOCACHE 指向 /go-cache——docker.rs 用共享 named volume 挂载，跨运行持久化
# （每次 go run 不再冷编译 stdlib，Go ≥1.20 不分发预编译 stdlib）；
# GOTMPDIR / GOPATH 指向可写的 /tmp tmpfs。
#
# 预热：构建期以默认编译 flag 编译一份导入常用纯 Go 标准库包的源码，把编译产物
# 烧进镜像层。首次创建空 named volume 时 Docker 会把挂载点内容 copy-up 进卷
# （未设 volume-nocopy），此后每次运行复用卷内缓存。只预热纯 Go 包：cgo 编译
# 需要 gcc，alpine 的 go 包不带 gcc（net/http 等 cgo 依赖包本就不支持，行为不变）。
# 预热以 root 执行、随后 chown 给 runner——卷内文件属主必须是 1000（运行时容器
# 以 1000:1000 运行并需写缓存）。
FROM yggdrasil-runner-base:latest

RUN apk add --no-cache go \
    && mkdir -p /go-cache /tmp/gopath \
    && chown -R runner:runner /go-cache /tmp/gopath

ENV GOCACHE=/go-cache \
    GOTMPDIR=/tmp \
    GOPATH=/tmp/gopath

RUN printf 'package main\nimport (\n    "bytes"\n    "encoding/json"\n    "fmt"\n    "math"\n    "os"\n    "reflect"\n    "sort"\n    "strconv"\n    "strings"\n    "time"\n)\nfunc main() {\n    _, _ = fmt.Println(strings.Join(os.Args, ","), time.Now().Unix(), strconv.Itoa(1), sort.IntsAreSorted([]int{1}), json.Valid([]byte("{}")), bytes.MinRead, math.Pi, reflect.TypeOf(0))\n}\n' > /tmp/warm.go \
    && go run /tmp/warm.go \
    && rm -f /tmp/warm.go \
    && chown -R runner:runner /go-cache

COPY runner.toml /runner.toml

USER runner
