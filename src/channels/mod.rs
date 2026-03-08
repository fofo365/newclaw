// Channel integration module

pub mod feishu;

pub use feishu::{
    FeishuConfig,
    FeishuMessage,
    FeishuEvent,
    FeishuClient,
    FeishuApiClient,
};
