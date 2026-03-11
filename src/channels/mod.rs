// Channel integration module

pub mod feishu;
pub mod feishu_stream;
pub mod feishu_file;
pub mod feishu_card;
pub mod feishu_user;
pub mod wecom;
pub mod qq;
pub mod telegram;
pub mod discord;
pub mod agp;

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
    CardElement as StreamCardElement,
    FeishuEventHandler,
    FeishuStreamEvent,
    FeishuResponse,
};

pub use feishu_file::{
    FeishuFileClient,
    FileType,
    UploadResult,
    ImageUploadResult,
    FileInfo,
    DownloadedFile,
    DownloadedImage,
};

pub use feishu_card::{
    FeishuCardClient,
    InteractiveCard,
    CardConfig,
    CardHeader,
    CardTitle,
    CardElement,
    CardAction,
    CardCallback,
    CardActionResponse,
    create_simple_card,
    create_card_with_buttons,
    create_card_with_dropdown,
};

pub use feishu_user::{
    FeishuUserClient,
    UserInfo,
    GroupInfo,
    GroupMember,
    GroupMemberList,
    PermissionInfo,
};

// WeCom (企业微信) exports
pub use wecom::{
    WeComConfig,
    WeComClient,
    WeComMessageClient,
    WeComWebhook,
    WeComCrypto,
    MessageTarget,
    MessageType,
    TextMessage,
    ImageMessage,
    FileMessage,
    VideoMessage,
    VoiceMessage,
    MediaType,
    WebhookInbound,
    WebhookTextMessage,
    WebhookEventMessage,
    create_client as create_wecom_client,
    create_message_client as create_wecom_message_client,
    create_webhook as create_wecom_webhook,
};

// QQ Bot exports
pub use qq::{
    QQConfig,
    QQClient,
    QQError,
};

// Telegram Bot exports
pub use telegram::{
    TelegramConfig,
    TelegramClient,
    TelegramError,
    User as TelegramUser,
    Message as TelegramMessage,
    Chat,
    PhotoSize,
    Document,
    WebhookInfo,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    CallbackQuery,
};

// Discord Bot exports
pub use discord::{
    DiscordConfig,
    DiscordClient,
    DiscordError,
    User as DiscordUser,
    Message as DiscordMessage,
    Command,
    CreateCommand,
    CommandOption,
    CommandOptionType,
    InteractionResponseType,
    InteractionResponseData,
    Interaction,
    InteractionType,
    InteractionData,
    Embed,
    EmbedField,
};

// AGP (Agent Gateway Protocol) Channel exports
pub use agp::{
    AGPConfig,
    AGPChannel,
    AGPMessage,
    FederationDomain,
    coordinator::{CoordinatorClient, EmbeddedCoordinator, AgentInfo, Registration},
    session::AGPSession,
    // Re-export types for external use
    Channel as AGPChannelTrait,
    Message as AGPMessage_,
    MessageHandler as AGPMessageHandler,
    ChannelError as AGPChannelError,
    ChannelHealth as AGPChannelHealth,
};
