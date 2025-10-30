#!/bin/bash

# Playwright UI 测试运行脚本

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║         Playwright UI 测试运行脚本                             ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# 检查 Playwright 是否安装
if ! npx playwright --version > /dev/null 2>&1; then
    echo "❌ Playwright 未安装，正在安装..."
    npx playwright install
fi

# 设置默认测试类型
TEST_TYPE="${1:-all}"
REPORTER="${2:-html}"

echo "📋 配置信息："
echo "  - 测试类型：$TEST_TYPE"
echo "  - 报告格式：$REPORTER"
echo "  - 时间：$(date)"
echo ""

# 运行测试
case $TEST_TYPE in
    all)
        echo "🚀 运行所有测试..."
        npx playwright test --reporter=$REPORTER
        ;;
    settings)
        echo "🚀 运行房间设置测试..."
        npx playwright test room-settings.spec.ts --reporter=$REPORTER
        ;;
    messaging)
        echo "🚀 运行消息系统测试..."
        npx playwright test messaging.spec.ts --reporter=$REPORTER
        ;;
    sample)
        echo "🚀 运行示例测试..."
        npx playwright test sample-room-tests.spec.ts --reporter=$REPORTER
        ;;
    ui)
        echo "🚀 运行测试 (UI 模式)..."
        npx playwright test --ui
        ;;
    debug)
        echo "🚀 运行测试 (调试模式)..."
        npx playwright test --debug
        ;;
    *)
        echo "❌ 未知的测试类型：$TEST_TYPE"
        echo ""
        echo "用法：$0 [all|settings|messaging|sample|ui|debug] [reporter]"
        echo ""
        echo "示例："
        echo "  $0 all                    # 运行所有测试"
        echo "  $0 settings               # 运行房间设置测试"
        echo "  $0 messaging              # 运行消息系统测试"
        echo "  $0 sample                 # 运行示例测试"
        echo "  $0 ui                     # UI 模式运行"
        echo "  $0 debug                  # 调试模式运行"
        exit 1
        ;;
esac

TEST_EXIT=$?

echo ""
echo "═══════════════════════════════════════════════════════════════"
if [ $TEST_EXIT -eq 0 ]; then
    echo "✅ 测试运行完成 - 所有测试通过！"
else
    echo "❌ 测试运行完成 - 存在失败的测试"
fi
echo "═══════════════════════════════════════════════════════════════"
echo ""

if [ "$REPORTER" = "html" ]; then
    echo "📊 查看 HTML 报告："
    echo "   open playwright-report/index.html"
fi

exit $TEST_EXIT
