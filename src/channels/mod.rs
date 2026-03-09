// Channel integration module

pub mod feishu;
pub mod feishu_stream;

pub use feishu::{
    FeishuConfig,
    FeishuMessage,
    FeishuEvent,
    FeishuClient,
    FeishuApiClient,
};

pub use feishu_stream::{
    FeishuStreamClient,
    RichTextContent,
    TextElement,
    TextStyle,
    CardContent,
    CardElement,
    FeishuEventHandler,
    FeishuStreamEvent,
    FeishuResponse,
};
