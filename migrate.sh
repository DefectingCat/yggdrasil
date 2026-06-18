#!/usr/bin/env bash
#
# 数据库迁移脚本（手动 / CI 备用）。
#
# 注意：服务器二进制现在会在启动时自动执行迁移（见 src/db/migrate.rs），
# 正常情况下无需手动运行本脚本。保留它是为了：
#   - 运维 escape hatch：二进制因迁移失败起不来时，手动救库
#   - CI/CD 中“先迁移再滚动发布”的工作流
#
# 本脚本与服务器内置运行器读取相同的 migrations/*.sql 文件，且这些 SQL
# 都是幂等的（IF NOT EXISTS / IF EXISTS / ON CONFLICT DO NOTHING），
# 因此两者混用安全。
#
set -euo pipefail

# 从 .env 加载环境变量
if [[ -f .env ]]; then
    echo "Loading DATABASE_URL from .env..."
    # shellcheck source=/dev/null
    set -a
    source .env
    set +a
fi

# 检查 DATABASE_URL
if [[ -z "${DATABASE_URL:-}" ]]; then
    echo "ERROR: DATABASE_URL is not set."
    echo "Please create a .env file or set the DATABASE_URL environment variable."
    echo "Example: export DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil"
    exit 1
fi

echo "Using database: $DATABASE_URL"

# 从 DATABASE_URL 中提取数据库名
DB_NAME=$(echo "$DATABASE_URL" | sed -n 's/.*\/\([^?]*\).*/\1/p')
# 提取不带数据库名的连接字符串（用于连接 postgres 系统库）
ADMIN_URL=$(echo "$DATABASE_URL" | sed "s|/${DB_NAME}|/postgres|")

echo "Target database: $DB_NAME"

# 检查目标数据库是否存在，不存在则创建
echo "Checking if database '$DB_NAME' exists..."
if ! psql "$ADMIN_URL" -tAc "SELECT 1 FROM pg_database WHERE datname='${DB_NAME}';" | grep -q 1; then
    echo "Database '$DB_NAME' does not exist. Creating..."
    psql "$ADMIN_URL" -c "CREATE DATABASE ${DB_NAME};"
    echo "Database '$DB_NAME' created."
else
    echo "Database '$DB_NAME' already exists."
fi

# 按顺序执行所有迁移文件
MIGRATIONS_DIR="migrations"

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo "ERROR: Migrations directory '$MIGRATIONS_DIR' not found."
    exit 1
fi

# 获取排序后的 SQL 文件列表
MIGRATION_FILES=""
for f in "$MIGRATIONS_DIR"/*.sql; do
    [[ -f "$f" ]] && MIGRATION_FILES="$MIGRATION_FILES$f
"
done

if [[ -z "$MIGRATION_FILES" ]]; then
    echo "No migration files found in $MIGRATIONS_DIR."
    exit 0
fi

echo ""
echo "Running migrations..."
echo "====================="

echo "$MIGRATION_FILES" | sort | while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    filename=$(basename "$file")
    echo -n "[$filename] ... "

    # 区分「已应用」与「真出错」：迁移本身已幂等（IF NOT EXISTS），正常应返回 0。
    # 非零退出码视为真错误，打印输出并中止，避免静默吞错（M6）。
    err_output=$(psql "$DATABASE_URL" -f "$file" 2>&1 >/dev/null)
    rc=$?

    if [[ $rc -eq 0 ]]; then
        echo "OK"
    elif echo "$err_output" | grep -qiE "already exists|duplicate|multiple primary keys"; then
        echo "SKIPPED (already applied)"
    else
        echo "FAIL"
        echo "$err_output" | head -5 | sed 's/^/  /'
        echo "Migration aborted due to error in $filename"
        exit 1
    fi
done

echo "====================="
echo "Migration complete!"
echo ""

# 显示当前数据库中的表
echo "Tables in database '$DB_NAME':"
psql "$DATABASE_URL" -c "\dt" || true
