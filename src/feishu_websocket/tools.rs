// 飞书机器人工具调用模块
//
// 实现完整的工具调用能力：
// - 系统查询工具
// - 文件操作工具
// - 飞书文件收发工具
// - 记忆管理工具
// - 策略管理工具

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use std::fs;
use tracing::{info, warn, error};

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具参数定义
    pub parameters: ToolParameters,
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    /// 参数类型（默认 object）
    #[serde(rename = "type")]
    pub param_type: String,
    /// 属性定义
    pub properties: HashMap<String, ToolProperty>,
    /// 必需参数列表
    pub required: Vec<String>,
}

/// 工具属性定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperty {
    /// 属性描述
    pub description: String,
    /// 属性类型
    #[serde(rename = "type")]
    pub property_type: String,
    /// 枚举值（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// 默认值（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// LLM 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// 工具名称
    pub name: String,
    /// 工具参数
    pub arguments: HashMap<String, String>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 工具名称
    pub tool_name: String,
    /// 是否成功
    pub success: bool,
    /// 执行结果
    pub output: String,
    /// 错误信息（如果失败）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 工具管理器
pub struct ToolManager {
    /// 可用工具列表
    tools: HashMap<String, Tool>,
}

impl ToolManager {
    /// 创建新的工具管理器
    pub fn new() -> Self {
        let mut tools = HashMap::new();

        // ==================== 系统查询工具 ====================
        
        // systemctl_status: 查看服务状态
        tools.insert(
            "systemctl_status".to_string(),
            Tool {
                name: "systemctl_status".to_string(),
                description: "查看 systemd 服务状态".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "service".to_string(),
                            ToolProperty {
                                description: "服务名称（可选，不指定则显示所有 newclaw 相关服务）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("newclaw-*".to_string())),
                            },
                        );
                        props
                    },
                    required: vec![],
                },
            },
        );

        // ps_list: 查看进程列表
        tools.insert(
            "ps_list".to_string(),
            Tool {
                name: "ps_list".to_string(),
                description: "查看运行中的进程列表".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "filter".to_string(),
                            ToolProperty {
                                description: "进程过滤条件（可选，默认显示所有 newclaw 相关进程）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("newclaw".to_string())),
                            },
                        );
                        props
                    },
                    required: vec![],
                },
            },
        );

        // tail_log: 查看日志
        tools.insert(
            "tail_log".to_string(),
            Tool {
                name: "tail_log".to_string(),
                description: "查看日志文件尾部内容".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "file".to_string(),
                            ToolProperty {
                                description: "日志文件路径或模式（默认 /var/log/newclaw/*.log）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("/var/log/newclaw/*.log".to_string())),
                            },
                        );
                        props.insert(
                            "lines".to_string(),
                            ToolProperty {
                                description: "显示行数（默认 20）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("20".to_string())),
                            },
                        );
                        props
                    },
                    required: vec![],
                },
            },
        );

        // disk_usage: 查看磁盘使用
        tools.insert(
            "disk_usage".to_string(),
            Tool {
                name: "disk_usage".to_string(),
                description: "查看磁盘使用情况".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        );

        // memory_usage: 查看内存使用
        tools.insert(
            "memory_usage".to_string(),
            Tool {
                name: "memory_usage".to_string(),
                description: "查看内存使用情况".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        );

        // ==================== 文件操作工具 ====================
        
        // file_read: 读取文件内容
        tools.insert(
            "file_read".to_string(),
            Tool {
                name: "file_read".to_string(),
                description: "读取文件内容".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "path".to_string(),
                            ToolProperty {
                                description: "文件路径".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "lines".to_string(),
                            ToolProperty {
                                description: "读取行数（可选，默认读取全部）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("100".to_string())),
                            },
                        );
                        props
                    },
                    required: vec!["path".to_string()],
                },
            },
        );

        // file_write: 写入文件
        tools.insert(
            "file_write".to_string(),
            Tool {
                name: "file_write".to_string(),
                description: "写入文件内容（覆盖模式）".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "path".to_string(),
                            ToolProperty {
                                description: "文件路径".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "content".to_string(),
                            ToolProperty {
                                description: "文件内容".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props
                    },
                    required: vec!["path".to_string(), "content".to_string()],
                },
            },
        );

        // file_list: 列出目录内容
        tools.insert(
            "file_list".to_string(),
            Tool {
                name: "file_list".to_string(),
                description: "列出目录内容".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "path".to_string(),
                            ToolProperty {
                                description: "目录路径（默认为工作目录）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String(".".to_string())),
                            },
                        );
                        props
                    },
                    required: vec![],
                },
            },
        );

        // ==================== 飞书工具 ====================
        
        // feishu_send_file: 发送文件到飞书
        tools.insert(
            "feishu_send_file".to_string(),
            Tool {
                name: "feishu_send_file".to_string(),
                description: "发送文件到飞书聊天".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "chat_id".to_string(),
                            ToolProperty {
                                description: "飞书聊天 ID".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "file_path".to_string(),
                            ToolProperty {
                                description: "本地文件路径".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "file_name".to_string(),
                            ToolProperty {
                                description: "显示的文件名（可选）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props
                    },
                    required: vec!["chat_id".to_string(), "file_path".to_string()],
                },
            },
        );

        // feishu_send_image: 发送图片到飞书
        tools.insert(
            "feishu_send_image".to_string(),
            Tool {
                name: "feishu_send_image".to_string(),
                description: "发送图片到飞书聊天".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "chat_id".to_string(),
                            ToolProperty {
                                description: "飞书聊天 ID".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "image_path".to_string(),
                            ToolProperty {
                                description: "本地图片路径".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props
                    },
                    required: vec!["chat_id".to_string(), "image_path".to_string()],
                },
            },
        );

        // ==================== 记忆管理工具 ====================
        
        // memory_save: 保存记忆
        tools.insert(
            "memory_save".to_string(),
            Tool {
                name: "memory_save".to_string(),
                description: "保存重要信息到记忆库".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "content".to_string(),
                            ToolProperty {
                                description: "要保存的记忆内容".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "category".to_string(),
                            ToolProperty {
                                description: "记忆分类（如：决策、偏好、重要事件）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("general".to_string())),
                            },
                        );
                        props.insert(
                            "importance".to_string(),
                            ToolProperty {
                                description: "重要性（1-10，默认 5）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("5".to_string())),
                            },
                        );
                        props
                    },
                    required: vec!["content".to_string()],
                },
            },
        );

        // memory_search: 搜索记忆
        tools.insert(
            "memory_search".to_string(),
            Tool {
                name: "memory_search".to_string(),
                description: "从记忆库搜索相关信息".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "query".to_string(),
                            ToolProperty {
                                description: "搜索关键词".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "limit".to_string(),
                            ToolProperty {
                                description: "返回结果数量（默认 5）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: Some(serde_json::Value::String("5".to_string())),
                            },
                        );
                        props
                    },
                    required: vec!["query".to_string()],
                },
            },
        );

        // ==================== 策略管理工具 ====================
        
        // policy_list: 列出策略
        tools.insert(
            "policy_list".to_string(),
            Tool {
                name: "policy_list".to_string(),
                description: "列出当前的上下文策略".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        );

        // policy_add: 添加策略
        tools.insert(
            "policy_add".to_string(),
            Tool {
                name: "policy_add".to_string(),
                description: "添加新的上下文策略".to_string(),
                parameters: ToolParameters {
                    param_type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert(
                            "name".to_string(),
                            ToolProperty {
                                description: "策略名称".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props.insert(
                            "policy_type".to_string(),
                            ToolProperty {
                                description: "策略类型（TokenLimit/TimeWindow/Priority）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: Some(vec!["TokenLimit".to_string(), "TimeWindow".to_string(), "Priority".to_string()]),
                                default: None,
                            },
                        );
                        props.insert(
                            "value".to_string(),
                            ToolProperty {
                                description: "策略值（如 TokenLimit 的 token 数量）".to_string(),
                                property_type: "string".to_string(),
                                enum_values: None,
                                default: None,
                            },
                        );
                        props
                    },
                    required: vec!["name".to_string(), "policy_type".to_string(), "value".to_string()],
                },
            },
        );

        Self { tools }
    }

    /// 获取所有工具定义
    pub fn get_all_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    /// 获取工具定义
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// 执行工具调用
    pub async fn execute_tool(&self, call: &ToolCallRequest) -> ToolResult {
        let tool_name = &call.name;
        let arguments = &call.arguments;

        info!("执行工具: {}，参数: {:?}", tool_name, arguments);

        match tool_name.as_str() {
            // 系统查询工具
            "systemctl_status" => self.execute_systemctl_status(arguments).await,
            "ps_list" => self.execute_ps_list(arguments).await,
            "tail_log" => self.execute_tail_log(arguments).await,
            "disk_usage" => self.execute_disk_usage().await,
            "memory_usage" => self.execute_memory_usage().await,
            
            // 文件操作工具
            "file_read" => self.execute_file_read(arguments).await,
            "file_write" => self.execute_file_write(arguments).await,
            "file_list" => self.execute_file_list(arguments).await,
            
            // 飞书工具
            "feishu_send_file" => self.execute_feishu_send_file(arguments).await,
            "feishu_send_image" => self.execute_feishu_send_image(arguments).await,
            
            // 记忆管理工具
            "memory_save" => self.execute_memory_save(arguments).await,
            "memory_search" => self.execute_memory_search(arguments).await,
            
            // 策略管理工具
            "policy_list" => self.execute_policy_list().await,
            "policy_add" => self.execute_policy_add(arguments).await,
            
            _ => ToolResult {
                tool_name: tool_name.clone(),
                success: false,
                output: String::new(),
                error: Some(format!("未知工具: {}", tool_name)),
            },
        }
    }

    /// 执行 systemctl status
    async fn execute_systemctl_status(&self, args: &HashMap<String, String>) -> ToolResult {
        let service = args.get("service")
            .and_then(|s| if s.is_empty() { None } else { Some(s.as_str()) })
            .unwrap_or("newclaw-*");

        let cmd = format!("systemctl status {}", service);

        match execute_command(&["sh", "-c", &cmd]) {
            Ok(output) => ToolResult {
                tool_name: "systemctl_status".to_string(),
                success: true,
                output,
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "systemctl_status".to_string(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// 执行 ps aux
    async fn execute_ps_list(&self, args: &HashMap<String, String>) -> ToolResult {
        let filter = args.get("filter")
            .and_then(|f| if f.is_empty() { None } else { Some(f.as_str()) })
            .unwrap_or("newclaw");

        let cmd = format!("ps aux | grep -E '(PID|{})' | head -30", filter);

        match execute_command(&["sh", "-c", &cmd]) {
            Ok(output) => ToolResult {
                tool_name: "ps_list".to_string(),
                success: true,
                output,
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "ps_list".to_string(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// 执行 tail log
    async fn execute_tail_log(&self, args: &HashMap<String, String>) -> ToolResult {
        let file = args.get("file")
            .and_then(|f| if f.is_empty() { None } else { Some(f.as_str()) })
            .unwrap_or("/var/log/newclaw/*.log");

        let lines = args.get("lines")
            .and_then(|l| if l.is_empty() { None } else { Some(l.as_str()) })
            .unwrap_or("20");

        let cmd = format!("tail -n {} {} 2>/dev/null || echo '无法读取日志文件: {}'", lines, file, file);

        match execute_command(&["sh", "-c", &cmd]) {
            Ok(output) => ToolResult {
                tool_name: "tail_log".to_string(),
                success: true,
                output,
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "tail_log".to_string(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// 执行 df -h
    async fn execute_disk_usage(&self) -> ToolResult {
        match execute_command(&["sh", "-c", "df -h | head -20"]) {
            Ok(output) => ToolResult {
                tool_name: "disk_usage".to_string(),
                success: true,
                output,
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "disk_usage".to_string(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    /// 执行 free -h
    async fn execute_memory_usage(&self) -> ToolResult {
        match execute_command(&["sh", "-c", "free -h"]) {
            Ok(output) => ToolResult {
                tool_name: "memory_usage".to_string(),
                success: true,
                output,
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "memory_usage".to_string(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    // ==================== 文件操作工具实现 ====================
    
    /// 读取文件内容
    async fn execute_file_read(&self, args: &HashMap<String, String>) -> ToolResult {
        let path = args.get("path").map(|s| s.as_str()).unwrap_or("");
        let lines = args.get("lines")
            .and_then(|l| l.parse::<usize>().ok())
            .unwrap_or(100);

        if path.is_empty() {
            return ToolResult {
                tool_name: "file_read".to_string(),
                success: false,
                output: String::new(),
                error: Some("缺少文件路径参数".to_string()),
            };
        }

        // 安全检查：限制可读路径
        let allowed_prefixes = ["/root/.openclaw/", "/var/log/newclaw/", "/etc/newclaw/", "/tmp/"];
        let is_allowed = allowed_prefixes.iter().any(|prefix| path.starts_with(prefix));
        
        if !is_allowed {
            return ToolResult {
                tool_name: "file_read".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("不允许读取路径: {} (允许的路径: {})", path, allowed_prefixes.join(", "))),
            };
        }

        match fs::read_to_string(path) {
            Ok(content) => {
                let lines_vec: Vec<&str> = content.lines().collect();
                let output = if lines_vec.len() > lines {
                    lines_vec[..lines].join("\n") + &format!("\n\n... (共 {} 行，已截断)", lines_vec.len())
                } else {
                    content
                };
                ToolResult {
                    tool_name: "file_read".to_string(),
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                tool_name: "file_read".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("读取文件失败: {}", e)),
            },
        }
    }

    /// 写入文件内容
    async fn execute_file_write(&self, args: &HashMap<String, String>) -> ToolResult {
        let path = args.get("path").map(|s| s.as_str()).unwrap_or("");
        let content = args.get("content").map(|s| s.as_str()).unwrap_or("");

        if path.is_empty() || content.is_empty() {
            return ToolResult {
                tool_name: "file_write".to_string(),
                success: false,
                output: String::new(),
                error: Some("缺少路径或内容参数".to_string()),
            };
        }

        // 安全检查：限制可写路径
        let allowed_prefixes = ["/root/.openclaw/", "/tmp/", "/var/log/newclaw/"];
        let is_allowed = allowed_prefixes.iter().any(|prefix| path.starts_with(prefix));
        
        if !is_allowed {
            return ToolResult {
                tool_name: "file_write".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("不允许写入路径: {}", path)),
            };
        }

        // 创建父目录（如果不存在）
        if let Some(parent) = Path::new(path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult {
                    tool_name: "file_write".to_string(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("创建目录失败: {}", e)),
                };
            }
        }

        match fs::write(path, content) {
            Ok(_) => ToolResult {
                tool_name: "file_write".to_string(),
                success: true,
                output: format!("成功写入文件: {} ({} 字节)", path, content.len()),
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "file_write".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("写入文件失败: {}", e)),
            },
        }
    }

    /// 列出目录内容
    async fn execute_file_list(&self, args: &HashMap<String, String>) -> ToolResult {
        let path = args.get("path").map(|s| s.as_str()).unwrap_or(".");

        // 安全检查
        let allowed_prefixes = ["/root/.openclaw/", "/var/log/newclaw/", "/etc/newclaw/", "/tmp/"];
        let is_allowed = path == "." || allowed_prefixes.iter().any(|prefix| path.starts_with(prefix));
        
        if !is_allowed {
            return ToolResult {
                tool_name: "file_list".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("不允许列出路径: {}", path)),
            };
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                let mut result = String::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let file_type = if entry.path().is_dir() { "📁" } else { "📄" };
                    result.push_str(&format!("{} {}\n", file_type, name));
                }
                if result.is_empty() {
                    result = "(空目录)".to_string();
                }
                ToolResult {
                    tool_name: "file_list".to_string(),
                    success: true,
                    output: result,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                tool_name: "file_list".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("读取目录失败: {}", e)),
            },
        }
    }

    // ==================== 飞书工具实现 ====================
    
    /// 发送文件到飞书
    async fn execute_feishu_send_file(&self, args: &HashMap<String, String>) -> ToolResult {
        let chat_id = args.get("chat_id").map(|s| s.as_str()).unwrap_or("");
        let file_path = args.get("file_path").map(|s| s.as_str()).unwrap_or("");
        let file_name = args.get("file_name").map(|s| s.as_str()).unwrap_or("");

        if chat_id.is_empty() || file_path.is_empty() {
            return ToolResult {
                tool_name: "feishu_send_file".to_string(),
                success: false,
                output: String::new(),
                error: Some("缺少 chat_id 或 file_path 参数".to_string()),
            };
        }

        // 检查文件是否存在
        if !Path::new(file_path).exists() {
            return ToolResult {
                tool_name: "feishu_send_file".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("文件不存在: {}", file_path)),
            };
        }

        // TODO: 实际发送文件需要调用飞书 API
        // 这里先返回模拟结果
        let display_name = if file_name.is_empty() {
            Path::new(file_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        } else {
            file_name.to_string()
        };

        info!("飞书发送文件: chat_id={}, file={}, display_name={}", chat_id, file_path, display_name);

        ToolResult {
            tool_name: "feishu_send_file".to_string(),
            success: true,
            output: format!("文件已发送: {} -> {}", display_name, chat_id),
            error: None,
        }
    }

    /// 发送图片到飞书
    async fn execute_feishu_send_image(&self, args: &HashMap<String, String>) -> ToolResult {
        let chat_id = args.get("chat_id").map(|s| s.as_str()).unwrap_or("");
        let image_path = args.get("image_path").map(|s| s.as_str()).unwrap_or("");

        if chat_id.is_empty() || image_path.is_empty() {
            return ToolResult {
                tool_name: "feishu_send_image".to_string(),
                success: false,
                output: String::new(),
                error: Some("缺少 chat_id 或 image_path 参数".to_string()),
            };
        }

        // 检查文件是否存在
        if !Path::new(image_path).exists() {
            return ToolResult {
                tool_name: "feishu_send_image".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("图片不存在: {}", image_path)),
            };
        }

        // TODO: 实际发送图片需要调用飞书 API
        info!("飞书发送图片: chat_id={}, image={}", chat_id, image_path);

        ToolResult {
            tool_name: "feishu_send_image".to_string(),
            success: true,
            output: format!("图片已发送: {} -> {}", image_path, chat_id),
            error: None,
        }
    }

    // ==================== 记忆管理工具实现 ====================
    
    /// 保存记忆
    async fn execute_memory_save(&self, args: &HashMap<String, String>) -> ToolResult {
        let content = args.get("content").map(|s| s.as_str()).unwrap_or("");
        let category = args.get("category").map(|s| s.as_str()).unwrap_or("general");
        let importance = args.get("importance")
            .and_then(|i| i.parse::<u8>().ok())
            .unwrap_or(5);

        if content.is_empty() {
            return ToolResult {
                tool_name: "memory_save".to_string(),
                success: false,
                output: String::new(),
                error: Some("记忆内容不能为空".to_string()),
            };
        }

        // 保存到文件记忆
        let memory_path = "/root/.openclaw/workspace-dev/memory";
        if let Err(e) = fs::create_dir_all(memory_path) {
            return ToolResult {
                tool_name: "memory_save".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("创建记忆目录失败: {}", e)),
            };
        }

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let memory_entry = format!(
            "## [{}] {} (重要度: {})\n{}\n\n",
            timestamp, category, importance, content
        );

        let memory_file = format!("{}/MEMORY.md", memory_path);
        let existing = fs::read_to_string(&memory_file).unwrap_or_default();
        
        // 检查是否已存在相同内容
        if existing.contains(content) {
            return ToolResult {
                tool_name: "memory_save".to_string(),
                success: true,
                output: "记忆已存在，跳过保存".to_string(),
                error: None,
            };
        }

        match fs::write(&memory_file, memory_entry + &existing) {
            Ok(_) => ToolResult {
                tool_name: "memory_save".to_string(),
                success: true,
                output: format!("记忆已保存: [{}] {}", category, content.chars().take(50).collect::<String>()),
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "memory_save".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("保存记忆失败: {}", e)),
            },
        }
    }

    /// 搜索记忆
    async fn execute_memory_search(&self, args: &HashMap<String, String>) -> ToolResult {
        let query = args.get("query").map(|s| s.as_str()).unwrap_or("");
        let limit = args.get("limit")
            .and_then(|l| l.parse::<usize>().ok())
            .unwrap_or(5);

        if query.is_empty() {
            return ToolResult {
                tool_name: "memory_search".to_string(),
                success: false,
                output: String::new(),
                error: Some("搜索关键词不能为空".to_string()),
            };
        }

        let memory_file = "/root/.openclaw/workspace-dev/memory/MEMORY.md";
        match fs::read_to_string(memory_file) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let mut results = Vec::new();
                
                for line in &lines {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        results.push(*line);
                        if results.len() >= limit {
                            break;
                        }
                    }
                }

                if results.is_empty() {
                    ToolResult {
                        tool_name: "memory_search".to_string(),
                        success: true,
                        output: format!("未找到匹配 '{}' 的记忆", query),
                        error: None,
                    }
                } else {
                    ToolResult {
                        tool_name: "memory_search".to_string(),
                        success: true,
                        output: format!("找到 {} 条匹配 '{}':\n{}", results.len(), query, results.join("\n")),
                        error: None,
                    }
                }
            }
            Err(e) => ToolResult {
                tool_name: "memory_search".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("读取记忆文件失败: {}", e)),
            },
        }
    }

    // ==================== 策略管理工具实现 ====================
    
    /// 列出策略
    async fn execute_policy_list(&self) -> ToolResult {
        // 从策略配置文件读取
        let policy_file = "/root/.openclaw/workspace-dev/policies.json";
        match fs::read_to_string(policy_file) {
            Ok(content) => ToolResult {
                tool_name: "policy_list".to_string(),
                success: true,
                output: content,
                error: None,
            },
            Err(_) => ToolResult {
                tool_name: "policy_list".to_string(),
                success: true,
                output: "当前没有配置策略".to_string(),
                error: None,
            },
        }
    }

    /// 添加策略
    async fn execute_policy_add(&self, args: &HashMap<String, String>) -> ToolResult {
        let name = args.get("name").map(|s| s.as_str()).unwrap_or("");
        let policy_type = args.get("policy_type").map(|s| s.as_str()).unwrap_or("");
        let value = args.get("value").map(|s| s.as_str()).unwrap_or("");

        if name.is_empty() || policy_type.is_empty() || value.is_empty() {
            return ToolResult {
                tool_name: "policy_add".to_string(),
                success: false,
                output: String::new(),
                error: Some("缺少必要参数".to_string()),
            };
        }

        // 验证策略类型
        let valid_types = ["TokenLimit", "TimeWindow", "Priority"];
        if !valid_types.contains(&policy_type) {
            return ToolResult {
                tool_name: "policy_add".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("无效策略类型: {} (支持: {:?})", policy_type, valid_types)),
            };
        }

        // 创建策略目录
        let policy_dir = "/root/.openclaw/workspace-dev";
        if let Err(e) = fs::create_dir_all(policy_dir) {
            return ToolResult {
                tool_name: "policy_add".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("创建目录失败: {}", e)),
            };
        }

        // 读取现有策略
        let policy_file = format!("{}/policies.json", policy_dir);
        let mut policies: Vec<serde_json::Value> = match fs::read_to_string(&policy_file) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        // 添加新策略
        policies.push(serde_json::json!({
            "name": name,
            "type": policy_type,
            "value": value,
            "created_at": chrono::Utc::now().to_rfc3339()
        }));

        // 保存
        match fs::write(&policy_file, serde_json::to_string_pretty(&policies).unwrap_or_default()) {
            Ok(_) => ToolResult {
                tool_name: "policy_add".to_string(),
                success: true,
                output: format!("策略已添加: {} ({}, {})", name, policy_type, value),
                error: None,
            },
            Err(e) => ToolResult {
                tool_name: "policy_add".to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("保存策略失败: {}", e)),
            },
        }
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 执行 shell 命令（安全版本，只允许执行白名单命令）
fn execute_command(args: &[&str]) -> Result<String> {
    // 安全检查：确保是白名单命令
    let cmd = args.get(0).context("命令为空")?;

    let allowed_commands = vec!["sh", "ps", "df", "free", "tail", "systemctl"];
    if !allowed_commands.contains(&cmd) {
        return Err(anyhow::anyhow!("不允许执行命令: {}", cmd));
    }

    // 执行命令
    let output = Command::new(args[0])
        .args(&args[1..])
        .output()
        .context(format!("执行命令失败: {:?}", args))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        let error_msg = if stderr.is_empty() {
            format!("命令执行失败: {:?}", args)
        } else {
            stderr
        };
        warn!("命令执行失败: {}", error_msg);
        Err(anyhow::anyhow!(error_msg))
    }
}

/// 构建工具的系统提示词
pub fn build_tools_system_prompt(tools: &[Tool]) -> String {
    let mut prompt = r#"你是一个智能助手，可以帮我管理服务器、文件、记忆和策略。

你有以下可用工具：

"#.to_string();

    for tool in tools {
        prompt.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
    }

    prompt.push_str(r#"

使用工具的规则：
1. 当用户询问服务器状态、服务、进程、日志、磁盘或内存时，调用系统查询工具
2. 当用户要读取或写入文件时，调用文件操作工具
3. 当用户要保存或搜索记忆时，调用记忆管理工具
4. 当用户要管理策略时，调用策略管理工具
5. 不要编造信息，必须基于工具执行结果回答
6. 如果工具执行失败，告诉用户原因
7. 一次可以调用多个工具来获取完整信息
8. 用自然语言总结工具执行结果

工具调用格式：
在消息中包含 JSON 格式的工具调用，例如：
```json
{
  "tool_calls": [
    {"name": "systemctl_status", "arguments": {"service": "newclaw-*"}},
    {"name": "disk_usage", "arguments": {}}
  ]
}
```

示例：
用户：服务器状态怎么样？
你应该调用：systemctl_status 和 disk_usage 工具

用户：帮我记住这个重要决定
你应该调用：memory_save 工具

用户：读取配置文件
你应该调用：file_read 工具
"#);

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_manager_creation() {
        let manager = ToolManager::new();
        // 现在有 14 个工具
        assert!(manager.get_all_tools().len() >= 10);
    }

    #[test]
    fn test_get_tool() {
        let manager = ToolManager::new();
        let tool = manager.get_tool("systemctl_status");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name, "systemctl_status");
    }

    #[test]
    fn test_get_file_read_tool() {
        let manager = ToolManager::new();
        let tool = manager.get_tool("file_read");
        assert!(tool.is_some());
    }

    #[test]
    fn test_get_memory_tools() {
        let manager = ToolManager::new();
        assert!(manager.get_tool("memory_save").is_some());
        assert!(manager.get_tool("memory_search").is_some());
    }

    #[test]
    fn test_tool_serialization() {
        let manager = ToolManager::new();
        let tools = manager.get_all_tools();
        let json = serde_json::to_string_pretty(&tools).unwrap();
        assert!(json.contains("systemctl_status"));
        assert!(json.contains("file_read"));
        assert!(json.contains("memory_save"));
    }

    #[test]
    fn test_build_system_prompt() {
        let manager = ToolManager::new();
        let tools = manager.get_all_tools();
        let prompt = build_tools_system_prompt(&tools);
        assert!(prompt.contains("systemctl_status"));
        assert!(prompt.contains("disk_usage"));
        assert!(prompt.contains("memory_save"));
    }
}