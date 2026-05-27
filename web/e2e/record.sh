#!/bin/bash

# Playwright E2E 测试录制脚本

# 设定漂亮的控制台颜色和样式
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║         ${BOLD}Playwright E2E 自动录制与脚本生成工具 ${NC}                  ${CYAN}║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# 获取脚本自身绝对路径所在的目录，以及 web 根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MANUAL_RECORDS_DIR="$SCRIPT_DIR/records/manual"

# 切换到 web 目录以确保 local node_modules 能够被正确加载
cd "$WEB_DIR" || {
    echo -e "${RED}❌ 错误：无法切换到 web 根目录 ($WEB_DIR)${NC}"
    exit 1
}

# 1. 检查并确保目标输出目录存在
if [ ! -d "$MANUAL_RECORDS_DIR" ]; then
    echo -e "${YELLOW}📁 创建录制脚本输出目录：$MANUAL_RECORDS_DIR...${NC}"
    mkdir -p "$MANUAL_RECORDS_DIR"
fi

# 2. 检查 Playwright 是否安装
if ! npx playwright --version > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️ Playwright 未检测到，正在为您安装/初始化...${NC}"
    npx playwright install
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Playwright 安装失败，请检查网络或 npm 配置 ${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}✅ Playwright 环境就绪！[$(npx playwright --version)]${NC}"
echo ""

# 3. 引导用户确定录制文件名
DEFAULT_FILE="recorded_$(date +%Y%m%d_%H%M%S)"
echo -e "${BOLD}请输入您想保存的测试脚本名称 ${NC} (无需输入后缀 .spec.ts)："
echo -e "💡 直接回车将使用默认时间戳名称：${BLUE}$DEFAULT_FILE${NC}"
read -p "✍️  文件名：" INPUT_NAME

# 处理文件名输入
if [ -z "$INPUT_NAME" ]; then
    FILE_NAME="$DEFAULT_FILE"
else
    # 过滤掉用户可能带上的 .spec.ts 或 .ts 后缀
    CLEANED_NAME=$(echo "$INPUT_NAME" | sed 's/\.spec\.ts$//' | sed 's/\.ts$//')
    # 移除非法字符，保持蛇形命名或中划线命名友好
    FILE_NAME=$(echo "$CLEANED_NAME" | sed 's/[^a-zA-Z0-9_-]/_/g')
fi

OUTPUT_FILE="$MANUAL_RECORDS_DIR/${FILE_NAME}.spec.ts"
RELATIVE_OUTPUT_PATH="web/e2e/records/manual/${FILE_NAME}.spec.ts"

echo ""
echo -e "🎯 ${GREEN}录制配置就绪：${NC}"
echo -e "   - 目标路径：${BLUE}$OUTPUT_FILE${NC}"
echo -e "   - 相对路径：${YELLOW}$RELATIVE_OUTPUT_PATH${NC}"
echo ""
echo -e "${YELLOW}🚀 正在唤醒 Playwright GUI 录制器...${NC}"
echo -e "💡 ${BOLD}提示 ${NC}: "
echo -e "   1. 在弹出的浏览器窗口中操作您的业务流程。"
echo -e "   2. Playwright Inspector 窗口会实时生成测试代码。"
echo -e "   3. 录制完毕后，${BOLD}直接关闭浏览器窗口 ${NC} 即可，脚本将自动保存并退出。"
echo ""

# 4. 执行 Playwright 录制
npx playwright codegen --output="$OUTPUT_FILE"
CODEGEN_EXIT=$?

# 5. 自动容错重试：如果执行失败，可能是因为本地 ms-playwright 浏览器缓存丢失或损坏
if [ $CODEGEN_EXIT -ne 0 ] && [ ! -f "$OUTPUT_FILE" ]; then
    echo ""
    echo -e "${YELLOW}⚠️ 录制器启动失败。检测到可能是 Playwright 浏览器未下载或被清理。${NC}"
    echo -e "${CYAN}🔧 正在自动为您修复并下载 Playwright 浏览器二进制文件 (npx playwright install)...${NC}"
    echo ""

    npx playwright install

    if [ $? -eq 0 ]; then
        echo ""
        echo -e "${GREEN}✅ 浏览器二进制文件下载成功！正在重新唤醒 Playwright GUI 录制器...${NC}"
        echo ""
        npx playwright codegen --output="$OUTPUT_FILE"
        CODEGEN_EXIT=$?
    else
        echo ""
        echo -e "${RED}❌ 自动安装浏览器失败。这可能是网络问题，请尝试手动运行 'npx playwright install' 解决依赖问题。${NC}"
    fi
fi


echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
if [ $CODEGEN_EXIT -eq 0 ] && [ -f "$OUTPUT_FILE" ]; then
    echo -e "${GREEN}🎉 录制成功！您的测试脚本已安全保存。${NC}"
    echo -e "📂 保存文件：${BOLD}${GREEN}$OUTPUT_FILE${NC}"
    echo -e "📝 您可以使用以下命令在未来运行此单项测试："
    echo -e "   ${CYAN}npx playwright test $RELATIVE_OUTPUT_PATH --headed${NC}"
else
    echo -e "${RED}⚠️ 录制未完成或被取消，或者未生成有效的脚本文件。${NC}"
fi
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

exit $CODEGEN_EXIT
