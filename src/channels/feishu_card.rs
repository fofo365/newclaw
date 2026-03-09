// NewClaw v0.4.0 - 飞书高级卡片交互
//
// 核心功能：
// 1. 交互卡片发送
// 2. 卡片回调处理
// 3. 卡片动态更新
// 4. 按钮点击处理
// 5. 下拉菜单
// 6. 跳转链接

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};

/// 飞书卡片客户端
pub struct FeishuCardClient {
    client: Client,
    base_url: String,
    app_id: String,
    app_secret: String,
    access_token: Option<String>,
}

/// 交互卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveCard {
    /// 卡片配置
    pub config: CardConfig,
    /// 卡片头部
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<CardHeader>,
    /// 卡片元素
    pub elements: Vec<CardElement>,
}

/// 卡片配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardConfig {
    /// 宽度模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wide_screen_mode: Option<bool>,
    /// 是否启用转发
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_forward: Option<bool>,
    /// 是否更新多卡片
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_multi: Option<bool>,
    /// 是否使用草稿模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_standard: Option<bool>,
}

/// 卡片头部
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardHeader {
    /// 标题
    pub title: CardTitle,
    /// 模板（颜色主题）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// 副标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<CardSubtitle>,
}

/// 卡片标题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTitle {
    /// 内容
    pub content: String,
    /// 标签
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<CardIcon>,
}

/// 卡片副标题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSubtitle {
    /// 内容
    pub content: String,
    /// 标签
    pub tag: String,
}

/// 卡片图标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardIcon {
    /// 图片 key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img_key: Option<String>,
    /// 图片 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 图标类型
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub icon_type: Option<String>,
}

/// 卡片元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum CardElement {
    /// 文本块
    #[serde(rename = "div")]
    Div {
        /// 文本内容
        text: CardText,
        /// 字段列表
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<CardField>>,
    },
    
    /// 分隔线
    #[serde(rename = "hr")]
    Hr,
    
    /// 笔记
    #[serde(rename = "note")]
    Note {
        /// 元素列表
        elements: Vec<NoteElement>,
    },
    
    /// 图片
    #[serde(rename = "img")]
    Img {
        /// 图片 key
        img_key: String,
        /// 图片模式
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
        /// 替代文本
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<CardAlt>,
        /// 宽度（像素）
        #[serde(skip_serializing_if = "Option::is_none")]
        preview_img_width: Option<i32>,
    },
    
    /// Markdown
    #[serde(rename = "markdown")]
    Markdown {
        /// 内容
        content: String,
        /// 字体大小
        #[serde(skip_serializing_if = "Option::is_none")]
        font_size: Option<String>,
        /// 文本对齐
        #[serde(skip_serializing_if = "Option::is_none")]
        text_align: Option<String>,
    },
    
    /// 操作按钮组
    #[serde(rename = "action")]
    Action {
        /// 操作列表
        actions: Vec<CardAction>,
        /// 布局方式
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<String>,
    },
    
    /// 表单容器
    #[serde(rename = "form")]
    Form {
        /// 表单元素
        elements: Vec<FormElement>,
        /// 表单名称
        name: String,
    },
    
    /// 折叠面板
    #[serde(rename = "collapsible_panel")]
    CollapsiblePanel {
        /// 面板标题
        header: PanelHeader,
        /// 面板元素
        elements: Vec<CardElement>,
        /// 展开状态
        #[serde(skip_serializing_if = "Option::is_none")]
        expanded: Option<bool>,
    },
    
    /// 分栏容器
    #[serde(rename = "column_set")]
    ColumnSet {
        /// 分栏列表
        columns: Vec<Column>,
        /// 布局方式
        #[serde(skip_serializing_if = "Option::is_none")]
        flex_mode: Option<String>,
        /// 背景样式
        #[serde(skip_serializing_if = "Option::is_none")]
        background_style: Option<String>,
    },
}

/// 卡片文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardText {
    /// 标签
    pub tag: String,
    /// 内容
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<CardIcon>,
}

/// 卡片字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardField {
    /// 是否为短字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_short: Option<bool>,
    /// 文本内容
    pub text: CardText,
}

/// 笔记元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum NoteElement {
    #[serde(rename = "plain_text")]
    PlainText { content: String },
    #[serde(rename = "lark_md")]
    LarkMd { content: String },
}

/// 替代文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAlt {
    /// 内容
    pub content: String,
    /// 标签
    pub tag: String,
}

/// 卡片操作（按钮）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAction {
    /// 标签
    pub tag: String,
    /// 文本内容
    pub text: CardText,
    /// 操作类型
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// URL（跳转链接）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 多义性值（回调标识）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// 交互配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm: Option<ConfirmDialog>,
}

/// 确认对话框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDialog {
    /// 标题
    pub title: CardTitle,
    /// 内容
    pub text: CardText,
    /// 确认按钮文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_text: Option<String>,
    /// 取消按钮文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_text: Option<String>,
}

/// 表单元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum FormElement {
    /// 输入框
    #[serde(rename = "input")]
    Input {
        /// 名称
        name: String,
        /// 占位符
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<CardText>,
        /// 默认值
        #[serde(skip_serializing_if = "Option::is_none")]
        default_value: Option<String>,
        /// 是否必填
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        /// 标签
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    
    /// 文本域
    #[serde(rename = "textarea")]
    Textarea {
        /// 名称
        name: String,
        /// 占位符
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<CardText>,
        /// 默认值
        #[serde(skip_serializing_if = "Option::is_none")]
        default_value: Option<String>,
        /// 是否必填
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        /// 标签
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    
    /// 日期选择器
    #[serde(rename = "date_picker")]
    DatePicker {
        /// 名称
        name: String,
        /// 占位符
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<CardText>,
        /// 默认值（时间戳）
        #[serde(skip_serializing_if = "Option::is_none")]
        initial_date: Option<String>,
    },
    
    /// 选择器（下拉菜单）
    #[serde(rename = "select_static")]
    SelectStatic {
        /// 名称
        name: String,
        /// 占位符
        placeholder: CardText,
        /// 选项列表
        options: Vec<SelectOption>,
        /// 默认选中的值
        #[serde(skip_serializing_if = "Option::is_none")]
        initial_option: Option<String>,
    },
    
    /// 多选器
    #[serde(rename = "select_multi_static")]
    SelectMultiStatic {
        /// 名称
        name: String,
        /// 占位符
        placeholder: CardText,
        /// 选项列表
        options: Vec<SelectOption>,
        /// 默认选中的值列表
        #[serde(skip_serializing_if = "Option::is_none")]
        initial_options: Option<Vec<String>>,
    },
}

/// 选择器选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    /// 选项文本
    pub text: CardText,
    /// 选项值
    pub value: String,
    /// 是否禁用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

/// 折叠面板头部
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelHeader {
    /// 标题
    pub title: CardTitle,
    /// 模板
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// 分栏
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    /// 分栏元素
    pub elements: Vec<CardElement>,
    /// 宽度权重
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// 垂直对齐方式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    /// 背景样式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_style: Option<String>,
}

/// 卡片回调
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardCallback {
    /// 回调类型
    pub action: CardCallbackAction,
    /// 触发的 open_id
    pub open_id: String,
    /// 触发的 user_id
    pub user_id: String,
    /// 触发的 union_id
    pub union_id: String,
    /// 触发时间（Unix 时间戳）
    pub trigger_time: String,
    /// 卡片 token
    pub token: String,
    /// 卡片 open_message_id
    pub open_message_id: String,
    /// 卡片 open_id
    pub card_open_id: String,
    /// 触发值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// 表单值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_value: Option<HashMap<String, serde_json::Value>>,
}

/// 卡片回调动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardCallbackAction {
    /// 表单提交
    SubmitForm,
    /// 点击按钮
    ClickButton,
    /// 选择选项
    SelectOption,
    /// 更新卡片
    UpdateCard,
    /// 上传文件
    UploadFile,
}

/// 卡片动作响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardActionResponse {
    /// 响应类型
    #[serde(rename = "type")]
    pub response_type: String,
    /// 响应数据
    pub data: CardActionData,
}

/// 卡片动作数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardActionData {
    /// 成功提示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toast: Option<ToastMessage>,
    /// 更新的卡片
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<InteractiveCard>,
}

/// Toast 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastMessage {
    /// 消息内容
    pub content: String,
    /// 消息类型
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub toast_type: Option<String>,
}

/// 卡片更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardUpdateRequest {
    /// 卡片 token
    pub token: String,
    /// 卡片内容
    pub card: InteractiveCard,
}

impl FeishuCardClient {
    /// 创建新的卡片客户端
    pub fn new(app_id: String, app_secret: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://open.feishu.cn/open-apis".to_string(),
            app_id,
            app_secret,
            access_token: None,
        }
    }
    
    /// 设置基础 URL（用于测试）
    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    /// 确保访问令牌有效
    pub async fn ensure_token(&mut self) -> Result<()> {
        if self.access_token.is_some() {
            return Ok(());
        }
        
        self.refresh_token().await
    }
    
    /// 刷新访问令牌
    async fn refresh_token(&mut self) -> Result<()> {
        let url = format!("{}/auth/v3/tenant_access_token/internal", self.base_url);
        
        let request_body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret,
        });
        
        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to request access token")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse token response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to get access token: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to get access token: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let token = json["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No token in response"))?
            .to_string();
        
        self.access_token = Some(token);
        info!("Successfully obtained access token");
        
        Ok(())
    }
    
    /// 发送交互卡片消息
    ///
    /// # 参数
    /// - `chat_id`: 会话 ID
    /// - `card`: 交互卡片内容
    ///
    /// # 返回
    /// - 消息 ID
    pub async fn send_interactive_card(
        &mut self,
        chat_id: &str,
        card: &InteractiveCard,
    ) -> Result<String> {
        self.ensure_token().await?;
        
        let url = format!("{}/im/v1/messages", self.base_url);
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let card_json = serde_json::to_string(card)
            .context("Failed to serialize card")?;
        
        let request_body = serde_json::json!({
            "receive_id_type": "chat_id",
            "receive_id": chat_id,
            "msg_type": "interactive",
            "content": serde_json::json!({
                "type": "template",
                "data": {
                    "template_id": card_json
                }
            })
        });
        
        debug!("Sending interactive card to chat: {}", chat_id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send card")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse card response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to send card: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to send card: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        let message_id = json["data"]["message_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No message_id in response"))?
            .to_string();
        
        info!("Successfully sent interactive card: {}", message_id);
        
        Ok(message_id)
    }
    
    /// 处理卡片回调
    ///
    /// # 参数
    /// - `callback`: 卡片回调数据
    ///
    /// # 返回
    /// - 卡片动作响应
    pub async fn handle_card_callback(
        &mut self,
        callback: CardCallback,
    ) -> Result<CardActionResponse> {
        info!("Handling card callback: {:?}", callback.action);
        
        // 根据回调类型处理
        match callback.action {
            CardCallbackAction::ClickButton => {
                // 处理按钮点击
                if let Some(value) = &callback.value {
                    debug!("Button clicked with value: {:?}", value);
                }
                
                Ok(CardActionResponse {
                    response_type: "card_action".to_string(),
                    data: CardActionData {
                        toast: Some(ToastMessage {
                            content: "操作成功".to_string(),
                            toast_type: Some("success".to_string()),
                        }),
                        card: None,
                    },
                })
            }
            
            CardCallbackAction::SubmitForm => {
                // 处理表单提交
                if let Some(form_value) = &callback.form_value {
                    debug!("Form submitted with values: {:?}", form_value);
                }
                
                Ok(CardActionResponse {
                    response_type: "card_action".to_string(),
                    data: CardActionData {
                        toast: Some(ToastMessage {
                            content: "表单提交成功".to_string(),
                            toast_type: Some("success".to_string()),
                        }),
                        card: None,
                    },
                })
            }
            
            CardCallbackAction::SelectOption => {
                // 处理选项选择
                if let Some(value) = &callback.value {
                    debug!("Option selected: {:?}", value);
                }
                
                Ok(CardActionResponse {
                    response_type: "card_action".to_string(),
                    data: CardActionData {
                        toast: None,
                        card: None,
                    },
                })
            }
            
            CardCallbackAction::UpdateCard => {
                // 处理卡片更新
                Ok(CardActionResponse {
                    response_type: "card_action".to_string(),
                    data: CardActionData {
                        toast: None,
                        card: None,
                    },
                })
            }
            
            CardCallbackAction::UploadFile => {
                // 处理文件上传
                Ok(CardActionResponse {
                    response_type: "card_action".to_string(),
                    data: CardActionData {
                        toast: Some(ToastMessage {
                            content: "文件上传成功".to_string(),
                            toast_type: Some("success".to_string()),
                        }),
                        card: None,
                    },
                })
            }
        }
    }
    
    /// 更新已发送的卡片
    ///
    /// # 参数
    /// - `token`: 卡片 token
    /// - `card`: 新的卡片内容
    ///
    /// # 返回
    /// - 是否成功
    pub async fn update_card(
        &mut self,
        token: &str,
        card: &InteractiveCard,
    ) -> Result<()> {
        self.ensure_token().await?;
        
        let url = format!("{}/interactive/v1/card/update", self.base_url);
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let request_body = CardUpdateRequest {
            token: token.to_string(),
            card: card.clone(),
        };
        
        debug!("Updating card with token: {}", token);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&request_body)
            .send()
            .await
            .context("Failed to update card")?;
        
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse card update response")?;
        
        if json["code"].as_i64() != Some(0) {
            error!("Failed to update card: {:?}", json);
            return Err(anyhow::anyhow!(
                "Failed to update card: {}",
                json["msg"].as_str().unwrap_or("Unknown error")
            ));
        }
        
        info!("Successfully updated card");
        
        Ok(())
    }
}

// ==================== 辅助函数 ====================

/// 创建简单的文本卡片
pub fn create_simple_card(title: &str, content: &str) -> InteractiveCard {
    InteractiveCard {
        config: CardConfig {
            wide_screen_mode: Some(true),
            enable_forward: Some(true),
            update_multi: None,
            is_standard: None,
        },
        header: Some(CardHeader {
            title: CardTitle {
                content: title.to_string(),
                tag: "plain_text".to_string(),
                icon: None,
            },
            template: Some("blue".to_string()),
            subtitle: None,
        }),
        elements: vec![
            CardElement::Div {
                text: CardText {
                    tag: "lark_md".to_string(),
                    content: content.to_string(),
                    icon: None,
                },
                fields: None,
            },
        ],
    }
}

/// 创建带按钮的卡片
pub fn create_card_with_buttons(
    title: &str,
    content: &str,
    buttons: Vec<(String, String, Option<serde_json::Value>)>, // (text, url, value)
) -> InteractiveCard {
    let actions: Vec<CardAction> = buttons
        .into_iter()
        .map(|(text, url, value)| {
            let is_url = url.starts_with("http");
            CardAction {
                tag: "button".to_string(),
                text: CardText {
                    tag: "plain_text".to_string(),
                    content: text,
                    icon: None,
                },
                action_type: if is_url {
                    Some("primary".to_string())
                } else {
                    Some("primary".to_string())
                },
                url: if is_url { Some(url) } else { None },
                value,
                confirm: None,
            }
        })
        .collect();
    
    InteractiveCard {
        config: CardConfig {
            wide_screen_mode: Some(true),
            enable_forward: Some(true),
            update_multi: None,
            is_standard: None,
        },
        header: Some(CardHeader {
            title: CardTitle {
                content: title.to_string(),
                tag: "plain_text".to_string(),
                icon: None,
            },
            template: Some("blue".to_string()),
            subtitle: None,
        }),
        elements: vec![
            CardElement::Div {
                text: CardText {
                    tag: "lark_md".to_string(),
                    content: content.to_string(),
                    icon: None,
                },
                fields: None,
            },
            CardElement::Action {
                actions,
                layout: Some("bisected".to_string()),
            },
        ],
    }
}

/// 创建带下拉菜单的卡片
pub fn create_card_with_dropdown(
    title: &str,
    content: &str,
    dropdown_name: &str,
    options: Vec<(String, String)>, // (text, value)
) -> InteractiveCard {
    InteractiveCard {
        config: CardConfig {
            wide_screen_mode: Some(true),
            enable_forward: Some(true),
            update_multi: None,
            is_standard: None,
        },
        header: Some(CardHeader {
            title: CardTitle {
                content: title.to_string(),
                tag: "plain_text".to_string(),
                icon: None,
            },
            template: Some("blue".to_string()),
            subtitle: None,
        }),
        elements: vec![
            CardElement::Div {
                text: CardText {
                    tag: "lark_md".to_string(),
                    content: content.to_string(),
                    icon: None,
                },
                fields: None,
            },
            CardElement::Form {
                name: "dropdown_form".to_string(),
                elements: vec![FormElement::SelectStatic {
                    name: dropdown_name.to_string(),
                    placeholder: CardText {
                        tag: "plain_text".to_string(),
                        content: "请选择...".to_string(),
                        icon: None,
                    },
                    options: options
                        .into_iter()
                        .map(|(text, value)| SelectOption {
                            text: CardText {
                                tag: "plain_text".to_string(),
                                content: text,
                                icon: None,
                            },
                            value,
                            disabled: None,
                        })
                        .collect(),
                    initial_option: None,
                }],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_simple_card() {
        let card = create_simple_card("Test Title", "Test Content");
        assert!(card.header.is_some());
        assert_eq!(card.elements.len(), 1);
    }
    
    #[test]
    fn test_create_card_with_buttons() {
        let card = create_card_with_buttons(
            "Title",
            "Content",
            vec![
                ("Button 1".to_string(), "https://example.com".to_string(), None),
                ("Button 2".to_string(), "action_2".to_string(), Some(serde_json::json!({"action": "2"}))),
            ],
        );
        
        assert!(card.header.is_some());
        assert_eq!(card.elements.len(), 2);
        
        // 检查按钮元素
        if let CardElement::Action { actions, .. } = &card.elements[1] {
            assert_eq!(actions.len(), 2);
            assert_eq!(actions[0].url, Some("https://example.com".to_string()));
        } else {
            panic!("Expected Action element");
        }
    }
    
    #[test]
    fn test_create_card_with_dropdown() {
        let card = create_card_with_dropdown(
            "Title",
            "Content",
            "selection",
            vec![
                ("Option 1".to_string(), "opt1".to_string()),
                ("Option 2".to_string(), "opt2".to_string()),
            ],
        );
        
        assert!(card.header.is_some());
        assert_eq!(card.elements.len(), 2);
        
        // 检查表单元素
        if let CardElement::Form { elements, .. } = &card.elements[1] {
            assert_eq!(elements.len(), 1);
            if let FormElement::SelectStatic { options, .. } = &elements[0] {
                assert_eq!(options.len(), 2);
            } else {
                panic!("Expected SelectStatic element");
            }
        } else {
            panic!("Expected Form element");
        }
    }
    
    #[test]
    fn test_card_callback_serialization() {
        let callback = CardCallback {
            action: CardCallbackAction::ClickButton,
            open_id: "ou_xxx".to_string(),
            user_id: "xxx".to_string(),
            union_id: "on_xxx".to_string(),
            trigger_time: "1234567890".to_string(),
            token: "token_xxx".to_string(),
            open_message_id: "om_xxx".to_string(),
            card_open_id: "card_xxx".to_string(),
            value: Some(serde_json::json!({"key": "value"})),
            form_value: None,
        };
        
        let json = serde_json::to_string(&callback).unwrap();
        assert!(json.contains("click_button"));
        assert!(json.contains("ou_xxx"));
    }
    
    #[test]
    fn test_card_action_response() {
        let response = CardActionResponse {
            response_type: "card_action".to_string(),
            data: CardActionData {
                toast: Some(ToastMessage {
                    content: "Success".to_string(),
                    toast_type: Some("success".to_string()),
                }),
                card: None,
            },
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("card_action"));
        assert!(json.contains("Success"));
    }
}
