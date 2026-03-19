# 测试执行命令

## 控制命令

### 暂停测试
```bash
sed -i 's/status: running/status: paused/' /root/newclaw/tests/TEST_CONTROL.yaml
sed -i 's/paused_at: null/paused_at: <timestamp>/' /root/newclaw/tests/TEST_CONTROL.yaml
```

### 继续测试
```bash
sed -i 's/status: paused/status: running/' /root/newclaw/tests/TEST_CONTROL.yaml
sed -i 's/paused_at: .*/paused_at: null/' /root/newclaw/tests/TEST_CONTROL.yaml
```

### 清除测试
```bash
sed -i 's/status: .*/status: cleared/' /root/newclaw/tests/TEST_CONTROL.yaml
sed -i 's/cleared: false/cleared: true/' /root/newclaw/tests/TEST_CONTROL.yaml
```

## 查看状态
```bash
cat /root/newclaw/tests/TEST_CONTROL.yaml
```

## 查看报告
```bash
cat /root/newclaw/tests/TEST_REPORT.md
```

## 查看日志
```bash
tail -f /root/newclaw/tests/test_logs/test_execution.log
```