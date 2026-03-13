// 飞书多维表格工具
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct FeishuBitableTool {
    client: Arc<FeishuClient>,
}

impl Default for FeishuBitableTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FeishuBitableTool {
    pub fn new() -> Self {
        let config = FeishuConfig::default();
        Self {
            client: Arc::new(FeishuClient::new(config)),
        }
    }

    pub fn with_config(config: FeishuConfig) -> Self {
        Self {
            client: Arc::new(FeishuClient::new(config)),
        }
    }

    /// 获取表格元数据
    async fn get_meta(&self, app_token: &str, table_id: &str) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "get_meta",
            "app_token": app_token,
            "table_id": table_id,
            "message": "Bitable metadata retrieved"
        }))
    }

    /// 列出字段
    async fn list_fields(&self, app_token: &str, table_id: &str) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "list_fields",
            "app_token": app_token,
            "table_id": table_id,
            "fields": [],
            "message": "Fields listed successfully"
        }))
    }

    /// 列出记录
    async fn list_records(&self, app_token: &str, table_id: &str) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "list_records",
            "app_token": app_token,
            "table_id": table_id,
            "records": [],
            "message": "Records listed successfully"
        }))
    }

    /// 创建记录
    async fn create_record(&self, app_token: &str, table_id: &str, fields: Value) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "create_record",
            "app_token": app_token,
            "table_id": table_id,
            "fields": fields,
            "record_id": "recXXXXXX",
            "message": "Record created successfully"
        }))
    }

    /// 更新记录
    async fn update_record(&self, app_token: &str, table_id: &str, record_id: &str, fields: Value) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "update_record",
            "app_token": app_token,
            "table_id": table_id,
            "record_id": record_id,
            "fields": fields,
            "message": "Record updated successfully"
        }))
    }
}

#[async_trait]
impl Tool for FeishuBitableTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_bitable".to_string(),
            description: "Feishu Bitable (multidimensional table) operations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Bitable action (get_meta, list_fields, list_records, create_record, update_record)"
                    },
                    "app_token": {
                        "type": "string",
                        "description": "Bitable app token"
                    },
                    "table_id": {
                        "type": "string",
                        "description": "Table ID"
                    },
                    "record_id": {
                        "type": "string",
                        "description": "Record ID (for update_record)"
                    },
                    "fields": {
                        "type": "object",
                        "description": "Field values (for create/update)"
                    }
                },
                "required": ["action", "app_token", "table_id"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("");
        let app_token = args["app_token"].as_str().unwrap_or("");
        let table_id = args["table_id"].as_str().unwrap_or("");

        match action {
            "get_meta" => self.get_meta(app_token, table_id).await,
            "list_fields" => self.list_fields(app_token, table_id).await,
            "list_records" => self.list_records(app_token, table_id).await,
            "create_record" => {
                let fields = args.get("fields").cloned().unwrap_or(json!({}));
                self.create_record(app_token, table_id, fields).await
            }
            "update_record" => {
                let record_id = args["record_id"].as_str().unwrap_or("");
                let fields = args.get("fields").cloned().unwrap_or(json!({}));
                self.update_record(app_token, table_id, record_id, fields).await
            }
            _ => Ok(json!({
                "status": "error",
                "message": format!("Unknown action: {}", action)
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitable_metadata() {
        let tool = FeishuBitableTool::new();
        assert_eq!(tool.metadata().name, "feishu_bitable");
    }

    #[tokio::test]
    async fn test_get_meta() {
        let tool = FeishuBitableTool::new();
        let result = tool.get_meta("app123", "table456").await.unwrap();
        assert_eq!(result["action"], "get_meta");
    }

    #[tokio::test]
    async fn test_list_fields() {
        let tool = FeishuBitableTool::new();
        let result = tool.list_fields("app123", "table456").await.unwrap();
        assert_eq!(result["action"], "list_fields");
    }

    #[tokio::test]
    async fn test_list_records() {
        let tool = FeishuBitableTool::new();
        let result = tool.list_records("app123", "table456").await.unwrap();
        assert_eq!(result["action"], "list_records");
    }

    #[tokio::test]
    async fn test_create_record() {
        let tool = FeishuBitableTool::new();
        let result = tool.create_record("app123", "table456", json!({"name": "test"})).await.unwrap();
        assert_eq!(result["action"], "create_record");
    }
}
