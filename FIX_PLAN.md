# 修复方案

## 当前状态

- **编译错误**: 8 个（稳定）
- **主要问题**: CLI 和 Gateway 引用不存在的工具

## 快速解决方案

### 临时禁用工具相关功能

1. **CLI 模块**：
   - 注释掉工具导入
   - 注释掉 register_tools 调用
   - 注释掉 list_tools 调用

2. **Gateway 模块**：
   - 注释掉 create_llm_provider 导入
   - 暂时使用占位符

3. **重新编译**

---

## 实施步骤

```bash
# 1. 备份当前文件
cd /root/newclaw
cp src/cli/mod.rs src/cli/mod.rs.bak2
cp src/gateway/mod.rs src/gateway/mod.rs.bak2

# 2. 注释掉 CLI 中的工具导入
sed -i '331s|.*use crate::tools.*ReadTool.*$|// &|' src/cli/mod.rs

# 3. 注释掉工具函数调用
sed -i 's|async fn register_tools|// TODO: async fn register_tools|' src/cli/mod.rs
sed -i 's|async fn list_tools|// TODO: async fn list_tools|' src/cli/mod.rs

# 4. 注释掉 Gateway 中的导入
sed -i 's|use crate::llm::create_llm_provider|// use crate::llm::create_llm_provider|' src/gateway/mod.rs

# 5. 重新编译
cargo build --lib
```

---

## 预期结果

编译错误应该减少到 0-2 个。

如果仍有错误，继续针对性修复。
