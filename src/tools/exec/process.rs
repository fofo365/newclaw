// 后台进程管理工具

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// 后台进程管理器
pub struct ProcessManager {
    /// 运行中的进程
    processes: Arc<RwLock<HashMap<String, u32>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// 后台进程工具
pub struct ProcessTool {
    manager: Arc<ProcessManager>,
}

impl ProcessTool {
    pub fn new(manager: Arc<ProcessManager>) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl Tool for ProcessTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "process".to_string(),
            description: "管理后台进程".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "list", "kill"],
                        "description": "操作类型"
                    },
                    "command": {
                        "type": "string",
                        "description": "要启动的命令（start 时必需）"
                    },
                    "name": {
                        "type": "string",
                        "description": "进程名称"
                    },
                    "pid": {
                        "type": "integer",
                        "description": "进程 ID（kill 时使用）"
                    }
                },
                "required": ["action"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 action 参数".to_string()))?;
        
        match action {
            "start" => self.start_process(&args).await,
            "list" => self.list_processes().await,
            "kill" => self.kill_process(&args).await,
            _ => Err(ToolError::InvalidArguments(format!("未知操作: {}", action))),
        }
    }
}

impl ProcessTool {
    async fn start_process(&self, args: &serde_json::Value) -> ToolResult<serde_json::Value> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 command 参数".to_string()))?;
        
        let name = args["name"]
            .as_str()
            .unwrap_or("unnamed");
        
        // 启动后台进程
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(command);
        
        let child = cmd.spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("启动进程失败: {}", e)))?;
        
        let pid = child.id().unwrap_or(0);
        
        // 记录进程
        let mut processes = self.manager.processes.write().await;
        processes.insert(name.to_string(), pid);
        
        Ok(serde_json::json!({
            "success": true,
            "name": name,
            "pid": pid
        }))
    }
    
    async fn list_processes(&self) -> ToolResult<serde_json::Value> {
        let processes = self.manager.processes.read().await;
        let list: Vec<_> = processes
            .iter()
            .map(|(name, pid)| {
                serde_json::json!({
                    "name": name,
                    "pid": pid
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "processes": list,
            "count": list.len()
        }))
    }
    
    async fn kill_process(&self, args: &serde_json::Value) -> ToolResult<serde_json::Value> {
        let pid = args["pid"]
            .as_u64()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 pid 参数".to_string()))? as i32;
        
        // 发送 SIGTERM
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        
        if result == 0 {
            // 从记录中移除
            let mut processes = self.manager.processes.write().await;
            processes.retain(|_, &mut p| p as i32 != pid);
            
            Ok(serde_json::json!({
                "success": true,
                "pid": pid
            }))
        } else {
            Err(ToolError::ExecutionFailed(format!("终止进程失败: {}", pid)))
        }
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_start_process() {
        let manager = Arc::new(ProcessManager::new());
        let tool = ProcessTool::new(manager.clone());
        
        let result = tool.execute(serde_json::json!({
            "action": "start",
            "command": "sleep 5",
            "name": "test_process"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["pid"].as_u64().unwrap() > 0);
    }
    
    #[tokio::test]
    async fn test_list_processes() {
        let manager = Arc::new(ProcessManager::new());
        let tool = ProcessTool::new(manager.clone());
        
        // 启动进程
        tool.execute(serde_json::json!({
            "action": "start",
            "command": "sleep 5",
            "name": "test1"
        })).await.unwrap();
        
        // 列出进程
        let result = tool.execute(serde_json::json!({
            "action": "list"
        })).await.unwrap();
        
        assert_eq!(result["count"], 1);
    }
}
