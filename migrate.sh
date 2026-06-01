#!/usr/bin/env bash
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

    if psql "$DATABASE_URL" -f "$file" > /dev/null 2>&1; then
        echo "OK"
    else
        echo "SKIPPED (already applied or error)"
    fi
done

echo "====================="
echo "Migration complete!"
echo ""

# 显示当前数据库中的表
echo "Tables in database '$DB_NAME':"
psql "$DATABASE_URL" -c "\dt" || true
