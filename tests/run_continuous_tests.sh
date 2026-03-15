#!/bin/bash
# NewClaw 持续测试执行脚本
# 用法: ./run_continuous_tests.sh

set -e

TEST_DIR="/root/newclaw/tests"
CONTROL_FILE="$TEST_DIR/TEST_CONTROL.yaml"
REPORT_FILE="$TEST_DIR/TEST_REPORT.md"
TEST_LOGS="$TEST_DIR/test_logs"

# 创建日志目录
mkdir -p "$TEST_LOGS"

# 日志函数
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$TEST_LOGS/test_execution.log"
}

# 检查是否应该暂停
check_pause() {
    if grep -q "status: paused" "$CONTROL_FILE" 2>/dev/null; then
        log "测试已暂停，等待继续..."
        return 1
    fi
    if grep -q "cleared: true" "$CONTROL_FILE" 2>/dev/null; then
        log "测试已清除，停止执行"
        return 1
    fi
    return 0
}

# 运行单元测试
run_unit_tests() {
    log "开始执行单元测试..."
    local log_file="$TEST_LOGS/unit_tests_$(date +%Y%m%d_%H%M%S).log"
    
    cd /root/newclaw
    cargo test --workspace --no-fail-fast 2>&1 | tee "$log_file"
    
    local exit_code=${PIPESTATUS[0]}
    if [ $exit_code -eq 0 ]; then
        log "单元测试通过"
        echo "PASSED" >> "$TEST_LOGS/unit_tests_results.txt"
    else
        log "单元测试有失败"
        echo "FAILED" >> "$TEST_LOGS/unit_tests_results.txt"
    fi
    
    return $exit_code
}

# 运行集成测试
run_integration_tests() {
    log "开始执行集成测试..."
    local log_file="$TEST_LOGS/integration_tests_$(date +%Y%m%d_%H%M%S).log"
    
    cd /root/newclaw
    cargo test --test '*' 2>&1 | tee "$log_file"
    
    local exit_code=${PIPESTATUS[0]}
    if [ $exit_code -eq 0 ]; then
        log "集成测试通过"
        echo "PASSED" >> "$TEST_LOGS/integration_tests_results.txt"
    else
        log "集成测试有失败"
        echo "FAILED" >> "$TEST_LOGS/integration_tests_results.txt"
    fi
    
    return $exit_code
}

# 主循环
main() {
    log "========== 持续测试开始 =========="
    
    # 初始化控制文件
    if [ ! -f "$CONTROL_FILE" ]; then
        log "错误: 控制文件不存在"
        exit 1
    fi
    
    # 执行测试阶段
    phases=("unit" "integration" "e2e" "regression" "performance")
    
    for phase in "${phases[@]}"; do
        check_pause || exit 0
        
        case $phase in
            "unit")
                run_unit_tests || true
                ;;
            "integration")
                run_integration_tests || true
                ;;
            "e2e")
                log "端到端测试待实现"
                ;;
            "regression")
                log "回归测试待实现"
                ;;
            "performance")
                log "性能测试待实现"
                ;;
        esac
    done
    
    log "========== 持续测试完成 =========="
}

main "$@"