#!/bin/bash
# 类型同步脚本
# 自动生成 TypeScript 类型定义并同步到前端

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKEND_DIR="$PROJECT_ROOT/crates/board"
FRONTEND_DIR="$PROJECT_ROOT/web"
TYPES_DIR="$FRONTEND_DIR/types/generated"

echo "🔄 开始同步 TypeScript 类型..."

# 检查后端目录
if [ ! -d "$BACKEND_DIR" ]; then
    echo "❌ 错误：后端目录不存在：$BACKEND_DIR"
    exit 1
fi

# 创建前端类型目录
mkdir -p "$TYPES_DIR"

# 生成 TypeScript 类型
echo "📝 生成 TypeScript 类型定义..."
cd "$BACKEND_DIR"
cargo build --package elizabeth-board --features typescript-export

# 检查生成的文件
if [ -d "target/types" ]; then
    echo "📦 复制生成的类型文件到前端..."
    cp -r target/types/*.ts "$TYPES_DIR/" 2>/dev/null || echo "⚠️  警告：没有生成新的类型文件"
else
    echo "⚠️  警告：target/types 目录不存在"
fi

# 验证前端类型文件
echo "🔍 验证前端类型文件..."
if [ -f "$TYPES_DIR/api.types.ts" ]; then
    echo "✅ api.types.ts 已存在"
else
    echo "❌ 错误：api.types.ts 不存在"
    exit 1
fi

# 运行前端类型检查
if [ -d "$FRONTEND_DIR" ]; then
    echo "🔍 运行前端类型检查..."
    cd "$FRONTEND_DIR"

    if command -v pnpm &> /dev/null; then
        pnpm tsc --noEmit || echo "⚠️  警告：TypeScript 类型检查发现错误"
    elif command -v npx &> /dev/null; then
        npx tsc --noEmit || echo "⚠️  警告：TypeScript 类型检查发现错误"
    else
        echo "⚠️  警告：没有找到 pnpm 或 npx，跳过类型检查"
    fi
fi

echo "✅ 类型同步完成！"
echo ""
echo "📋 生成的类型文件位于：$TYPES_DIR"
echo "🔧 要重新生成类型，运行："
echo "   cd $BACKEND_DIR && cargo build --package elizabeth-board --features typescript-export"
