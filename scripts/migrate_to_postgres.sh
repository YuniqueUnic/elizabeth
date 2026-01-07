#!/bin/bash
# ============================================================================
# Elizabeth 数据库迁移脚本
# ============================================================================
# 用途：从 SQLite 迁移数据到 PostgreSQL
# 使用方法：./scripts/migrate_to_postgres.sh
#
# 环境变量：
#   SOURCE_DB - SQLite 数据库路径（默认：./elizabeth.db）
#   TARGET_DB - PostgreSQL 连接字符串（必需）
#   EXPORT_DIR - 导出文件目录（默认：./migrate_export）
#
# 示例：
#   export TARGET_DB="postgresql://user:password@localhost:5432/elizabeth" # pragma: allowlist secret
#   ./scripts/migrate_to_postgres.sh
# ============================================================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 默认配置
SOURCE_DB="${SOURCE_DB:-./elizabeth.db}"
TARGET_DB="${TARGET_DB:-}"
EXPORT_DIR="${EXPORT_DIR:-./migrate_export}"
MIGRATE_BIN="${MIGRATE_BIN:-./target/debug/migrate_db}"

# 检查必需的环境变量
if [ -z "$TARGET_DB" ]; then
    echo -e "${RED}错误：TARGET_DB 环境变量未设置 ${NC}"
    echo "使用方法：export TARGET_DB=\"postgresql://user:password@localhost:5432/elizabeth\"" # pragma: allowlist secret
    echo "         ./scripts/migrate_to_postgres.sh"
    exit 1
fi

# 检查源数据库是否存在
if [ ! -f "$SOURCE_DB" ]; then
    echo -e "${RED}错误：源数据库文件不存在：$SOURCE_DB${NC}"
    exit 1
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Elizabeth 数据库迁移工具 ${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "源数据库：$SOURCE_DB"
echo "目标数据库：$TARGET_DB"
echo "导出目录：$EXPORT_DIR"
echo ""

# 创建导出目录
mkdir -p "$EXPORT_DIR"

# 步骤 1: 检查迁移程序是否存在
echo -e "${YELLOW}步骤 1: 检查迁移程序...${NC}"
if [ ! -f "$MIGRATE_BIN" ]; then
    echo -e "${YELLOW}迁移程序不存在，正在编译...${NC}"
    cargo build --bin migrate_db
    if [ $? -ne 0 ]; then
        echo -e "${RED}错误：编译迁移程序失败 ${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}✓ 迁移程序就绪 ${NC}"
echo ""

# 步骤 2: 导出 SQLite 数据
echo -e "${YELLOW}步骤 2: 导出 SQLite 数据...${NC}"
$MIGRATE_BIN export \
    --source "$SOURCE_DB" \
    --output "$EXPORT_DIR/data.json"

if [ $? -ne 0 ]; then
    echo -e "${RED}错误：导出数据失败 ${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 数据导出完成 ${NC}"
echo ""

# 步骤 3: 导入到 PostgreSQL
echo -e "${YELLOW}步骤 3: 导入数据到 PostgreSQL...${NC}"
$MIGRATE_BIN import \
    --target "$TARGET_DB" \
    --input "$EXPORT_DIR/data.json"

if [ $? -ne 0 ]; then
    echo -e "${RED}错误：导入数据失败 ${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 数据导入完成 ${NC}"
echo ""

# 步骤 4: 验证数据
echo -e "${YELLOW}步骤 4: 验证数据完整性...${NC}"
$MIGRATE_BIN verify \
    --source "$SOURCE_DB" \
    --target "$TARGET_DB"

if [ $? -ne 0 ]; then
    echo -e "${YELLOW}警告：数据验证发现问题，请检查日志 ${NC}"
else
    echo -e "${GREEN}✓ 数据验证通过 ${NC}"
fi
echo ""

# 完成
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}迁移完成!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "后续步骤："
echo "1. 更新配置文件中的 DATABASE_URL"
echo "2. 重启应用程序"
echo "3. 验证所有功能正常"
echo "4. 备份原 SQLite 数据库"
echo ""
echo "清理临时文件："
echo "  rm -rf $EXPORT_DIR"
