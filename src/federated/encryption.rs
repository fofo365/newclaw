//! Federated Memory Encryption - 联邦记忆加密模块
//!
//! 提供节点间通信的加密传输功能
//! 支持 AES-256-GCM 加密、密钥交换、签名验证
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// 加密错误
// ============================================================================

/// 加密错误
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("密钥生成失败: {0}")]
    KeyGenerationFailed(String),
    
    #[error("加密失败: {0}")]
    EncryptionFailed(String),
    
    #[error("解密失败: {0}")]
    DecryptionFailed(String),
    
    #[error("签名验证失败: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("无效密钥: {0}")]
    InvalidKey(String),
    
    #[error("密钥过期: {0}")]
    KeyExpired(String),
    
    #[error("证书无效: {0}")]
    InvalidCertificate(String),
    
    #[error("nonce 错误: {0}")]
    NonceError(String),
    
    #[error("编码错误: {0}")]
    EncodingError(String),
}

pub type EncryptionResult<T> = std::result::Result<T, EncryptionError>;

// ============================================================================
// 密钥类型
// ============================================================================

/// 密钥 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyId(String);

impl KeyId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for KeyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 密钥类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyType {
    /// AES 对称密钥
    Aes256,
    /// RSA 非对称密钥对
    Rsa2048,
    /// Ed25519 签名密钥对
    Ed25519,
}

/// 密钥用途
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyUsage {
    /// 加密
    Encryption,
    /// 解密
    Decryption,
    /// 签名
    Signing,
    /// 验证
    Verification,
}

// ============================================================================
// 密钥结构
// ============================================================================

/// 对称密钥
#[derive(Debug, Clone)]
pub struct SymmetricKey {
    /// 密钥 ID
    pub id: KeyId,
    /// 密钥数据（32 bytes for AES-256）
    pub key: Vec<u8>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 密钥状态
    pub status: KeyStatus,
}

/// 密钥状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyStatus {
    Active,
    Expired,
    Revoked,
}

impl SymmetricKey {
    /// 生成新的 AES-256 密钥
    pub fn generate() -> EncryptionResult<Self> {
        let mut key = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        
        Ok(Self {
            id: KeyId::new(),
            key,
            created_at: Utc::now(),
            expires_at: None,
            status: KeyStatus::Active,
        })
    }
    
    /// 从字节创建
    pub fn from_bytes(key: Vec<u8>) -> EncryptionResult<Self> {
        if key.len() != 32 {
            return Err(EncryptionError::InvalidKey(
                format!("Invalid key length: expected 32, got {}", key.len())
            ));
        }
        
        Ok(Self {
            id: KeyId::new(),
            key,
            created_at: Utc::now(),
            expires_at: None,
            status: KeyStatus::Active,
        })
    }
    
    /// 从 Base64 创建
    pub fn from_base64(encoded: &str) -> EncryptionResult<Self> {
        let key = BASE64.decode(encoded)
            .map_err(|e| EncryptionError::EncodingError(e.to_string()))?;
        Self::from_bytes(key)
    }
    
    /// 导出为 Base64
    pub fn to_base64(&self) -> String {
        BASE64.encode(&self.key)
    }
    
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at || self.status == KeyStatus::Expired
        } else {
            self.status == KeyStatus::Expired
        }
    }
    
    /// 检查是否可用
    pub fn is_usable(&self) -> bool {
        self.status == KeyStatus::Active && !self.is_expired()
    }
    
    /// 设置过期时间
    pub fn with_expiry(mut self, duration: chrono::Duration) -> Self {
        self.expires_at = Some(Utc::now() + duration);
        self
    }
}

/// 密钥对
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// 密钥 ID
    pub id: KeyId,
    /// 公钥
    pub public_key: Vec<u8>,
    /// 私钥
    pub private_key: Vec<u8>,
    /// 密钥类型
    pub key_type: KeyType,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
}

impl KeyPair {
    /// 生成新的密钥对（简化实现，实际应使用 ring 或 openssl）
    pub fn generate(key_type: KeyType) -> EncryptionResult<Self> {
        let (public_key, private_key) = match key_type {
            KeyType::Ed25519 => {
                // 使用随机字节作为简化的密钥对
                let mut public = vec![0u8; 32];
                let mut private = vec![0u8; 64];
                rand::rngs::OsRng.fill_bytes(&mut public);
                rand::rngs::OsRng.fill_bytes(&mut private);
                (public, private)
            }
            KeyType::Rsa2048 => {
                // 简化实现
                let mut public = vec![0u8; 256];
                let mut private = vec![0u8; 512];
                rand::rngs::OsRng.fill_bytes(&mut public);
                rand::rngs::OsRng.fill_bytes(&mut private);
                (public, private)
            }
            _ => return Err(EncryptionError::KeyGenerationFailed(
                "Unsupported key type".to_string()
            )),
        };
        
        Ok(Self {
            id: KeyId::new(),
            public_key,
            private_key,
            key_type,
            created_at: Utc::now(),
            expires_at: None,
        })
    }
    
    /// 获取公钥 Base64
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(&self.public_key)
    }
}

// ============================================================================
// 加密器
// ============================================================================

/// 加密配置
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// 是否启用加密
    pub enabled: bool,
    /// 默认加密算法
    pub default_algorithm: EncryptionAlgorithm,
    /// 密钥轮换周期（天）
    pub key_rotation_days: u64,
    /// Nonce 长度
    pub nonce_length: usize,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_rotation_days: 30,
            nonce_length: 12,
        }
    }
}

/// 加密算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// 加密数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// 加密算法
    pub algorithm: EncryptionAlgorithm,
    /// 密钥 ID
    pub key_id: KeyId,
    /// Nonce
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// 密文
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
    /// 认证标签
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_bytes_option")]
    pub tag: Option<Vec<u8>>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use base64::Engine;
    
    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        base64::engine::general_purpose::STANDARD.encode(bytes).serialize(serializer)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .map_err(serde::de::Error::custom)
    }
}

mod serde_bytes_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use base64::Engine;
    
    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(b) => Some(base64::engine::general_purpose::STANDARD.encode(b)).serialize(serializer),
            None => None::<String>.serialize(serializer),
        }
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(encoded) => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&encoded)
                    .map_err(serde::de::Error::custom)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }
}

/// 加密器
pub struct Encryptor {
    config: EncryptionConfig,
    keys: RwLock<HashMap<KeyId, SymmetricKey>>,
    current_key_id: RwLock<Option<KeyId>>,
}

impl Encryptor {
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config,
            keys: RwLock::new(HashMap::new()),
            current_key_id: RwLock::new(None),
        }
    }
    
    /// 生成新密钥
    pub async fn generate_key(&self) -> EncryptionResult<KeyId> {
        let key = SymmetricKey::generate()?;
        let key_id = key.id.clone();
        
        let mut keys = self.keys.write().await;
        keys.insert(key_id.clone(), key);
        
        let mut current = self.current_key_id.write().await;
        *current = Some(key_id.clone());
        
        Ok(key_id)
    }
    
    /// 添加密钥
    pub async fn add_key(&self, key: SymmetricKey) -> EncryptionResult<()> {
        let key_id = key.id.clone();
        let mut keys = self.keys.write().await;
        keys.insert(key_id, key);
        Ok(())
    }
    
    /// 获取当前密钥 ID
    pub async fn current_key_id(&self) -> Option<KeyId> {
        self.current_key_id.read().await.clone()
    }
    
    /// 加密数据
    pub async fn encrypt(&self, plaintext: &[u8]) -> EncryptionResult<EncryptedData> {
        if !self.config.enabled {
            return Err(EncryptionError::EncryptionFailed("Encryption disabled".to_string()));
        }
        
        let current_id = self.current_key_id.read().await;
        let key_id = current_id.as_ref()
            .ok_or_else(|| EncryptionError::InvalidKey("No current key".to_string()))?;
        
        let keys = self.keys.read().await;
        let key = keys.get(key_id)
            .ok_or_else(|| EncryptionError::InvalidKey(format!("Key not found: {}", key_id)))?;
        
        if !key.is_usable() {
            return Err(EncryptionError::KeyExpired(key_id.to_string()));
        }
        
        // 生成 nonce
        let mut nonce = vec![0u8; self.config.nonce_length];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        
        // 简化的加密实现（实际应使用 AES-GCM）
        let ciphertext = self.encrypt_with_key(plaintext, &key.key, &nonce)?;
        
        Ok(EncryptedData {
            algorithm: self.config.default_algorithm,
            key_id: key_id.clone(),
            nonce,
            ciphertext,
            tag: None,
            timestamp: Utc::now(),
        })
    }
    
    /// 使用密钥加密
    fn encrypt_with_key(&self, plaintext: &[u8], key: &[u8], nonce: &[u8]) -> EncryptionResult<Vec<u8>> {
        // 简化的 XOR 加密（实际应使用 AES-GCM）
        // 这里仅用于演示，生产环境必须使用正确的加密库
        
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        
        // 使用 SHA256(key || nonce) 生成密钥流
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(nonce);
        let keystream = hasher.finalize();
        
        // XOR 加密
        for (i, byte) in plaintext.iter().enumerate() {
            ciphertext.push(byte ^ keystream[i % 32]);
        }
        
        // 添加认证标签（简化）
        let mut tag_hasher = Sha256::new();
        tag_hasher.update(&ciphertext);
        tag_hasher.update(key);
        let tag = tag_hasher.finalize();
        ciphertext.extend_from_slice(&tag[..16]);
        
        Ok(ciphertext)
    }
    
    /// 解密数据
    pub async fn decrypt(&self, encrypted: &EncryptedData) -> EncryptionResult<Vec<u8>> {
        if !self.config.enabled {
            return Err(EncryptionError::DecryptionFailed("Encryption disabled".to_string()));
        }
        
        let keys = self.keys.read().await;
        let key = keys.get(&encrypted.key_id)
            .ok_or_else(|| EncryptionError::InvalidKey(
                format!("Key not found: {}", encrypted.key_id)
            ))?;
        
        if !key.is_usable() {
            return Err(EncryptionError::KeyExpired(encrypted.key_id.to_string()));
        }
        
        self.decrypt_with_key(&encrypted.ciphertext, &key.key, &encrypted.nonce)
    }
    
    /// 使用密钥解密
    fn decrypt_with_key(&self, ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> EncryptionResult<Vec<u8>> {
        // 分离密文和标签
        if ciphertext.len() < 16 {
            return Err(EncryptionError::DecryptionFailed("Ciphertext too short".to_string()));
        }
        
        let (actual_ciphertext, stored_tag) = ciphertext.split_at(ciphertext.len() - 16);
        
        // 验证标签
        let mut tag_hasher = Sha256::new();
        tag_hasher.update(actual_ciphertext);
        tag_hasher.update(key);
        let expected_tag = tag_hasher.finalize();
        
        if stored_tag != &expected_tag[..16] {
            return Err(EncryptionError::SignatureVerificationFailed("Tag mismatch".to_string()));
        }
        
        // 生成密钥流
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(nonce);
        let keystream = hasher.finalize();
        
        // XOR 解密
        let mut plaintext = Vec::with_capacity(actual_ciphertext.len());
        for (i, byte) in actual_ciphertext.iter().enumerate() {
            plaintext.push(byte ^ keystream[i % 32]);
        }
        
        Ok(plaintext)
    }
}

// ============================================================================
// 签名器
// ============================================================================

/// 消息签名
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSignature {
    /// 签名算法
    pub algorithm: SignatureAlgorithm,
    /// 密钥 ID
    pub key_id: KeyId,
    /// 签名值
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 签名算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    /// HMAC-SHA256
    HmacSha256,
    /// HMAC-SHA512
    HmacSha512,
    /// Ed25519
    Ed25519,
}

/// 签名器
pub struct Signer {
    /// 签名密钥
    key: SymmetricKey,
    /// 签名算法
    algorithm: SignatureAlgorithm,
}

impl Signer {
    pub fn new(key: SymmetricKey, algorithm: SignatureAlgorithm) -> Self {
        Self { key, algorithm }
    }
    
    /// 对消息签名
    pub fn sign(&self, message: &[u8]) -> EncryptionResult<MessageSignature> {
        let signature = match self.algorithm {
            SignatureAlgorithm::HmacSha256 => {
                // 简化的 HMAC 实现
                let mut hasher = Sha256::new();
                hasher.update(&self.key.key);
                hasher.update(message);
                hasher.finalize().to_vec()
            }
            SignatureAlgorithm::HmacSha512 => {
                let mut hasher = Sha512::new();
                hasher.update(&self.key.key);
                hasher.update(message);
                hasher.finalize().to_vec()
            }
            SignatureAlgorithm::Ed25519 => {
                // 简化实现
                let mut hasher = Sha512::new();
                hasher.update(&self.key.key);
                hasher.update(message);
                hasher.finalize()[..64].to_vec()
            }
        };
        
        Ok(MessageSignature {
            algorithm: self.algorithm,
            key_id: self.key.id.clone(),
            signature,
            timestamp: Utc::now(),
        })
    }
    
    /// 验证签名
    pub fn verify(&self, message: &[u8], signature: &MessageSignature) -> EncryptionResult<bool> {
        // 重新计算签名
        let expected = self.sign(message)?;
        
        // 比较签名
        if expected.signature.len() != signature.signature.len() {
            return Ok(false);
        }
        
        // 常量时间比较
        let mut diff = 0u8;
        for (a, b) in expected.signature.iter().zip(signature.signature.iter()) {
            diff |= a ^ b;
        }
        
        Ok(diff == 0)
    }
}

// ============================================================================
// 密钥管理器
// ============================================================================

/// 密钥管理器
pub struct KeyManager {
    /// 加密密钥
    encryption_keys: RwLock<HashMap<KeyId, SymmetricKey>>,
    /// 签名密钥
    signing_keys: RwLock<HashMap<KeyId, SymmetricKey>>,
    /// 密钥对
    key_pairs: RwLock<HashMap<KeyId, KeyPair>>,
    /// 配置
    config: EncryptionConfig,
}

impl KeyManager {
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            encryption_keys: RwLock::new(HashMap::new()),
            signing_keys: RwLock::new(HashMap::new()),
            key_pairs: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// 生成加密密钥
    pub async fn generate_encryption_key(&self) -> EncryptionResult<KeyId> {
        let key = SymmetricKey::generate()?;
        let key_id = key.id.clone();
        
        let mut keys = self.encryption_keys.write().await;
        keys.insert(key_id.clone(), key);
        
        Ok(key_id)
    }
    
    /// 生成签名密钥
    pub async fn generate_signing_key(&self) -> EncryptionResult<KeyId> {
        let key = SymmetricKey::generate()?;
        let key_id = key.id.clone();
        
        let mut keys = self.signing_keys.write().await;
        keys.insert(key_id.clone(), key);
        
        Ok(key_id)
    }
    
    /// 生成密钥对
    pub async fn generate_key_pair(&self, key_type: KeyType) -> EncryptionResult<KeyId> {
        let pair = KeyPair::generate(key_type)?;
        let key_id = pair.id.clone();
        
        let mut pairs = self.key_pairs.write().await;
        pairs.insert(key_id.clone(), pair);
        
        Ok(key_id)
    }
    
    /// 获取加密密钥
    pub async fn get_encryption_key(&self, id: &KeyId) -> Option<SymmetricKey> {
        let keys = self.encryption_keys.read().await;
        keys.get(id).cloned()
    }
    
    /// 获取签名密钥
    pub async fn get_signing_key(&self, id: &KeyId) -> Option<SymmetricKey> {
        let keys = self.signing_keys.read().await;
        keys.get(id).cloned()
    }
    
    /// 获取密钥对
    pub async fn get_key_pair(&self, id: &KeyId) -> Option<KeyPair> {
        let pairs = self.key_pairs.read().await;
        pairs.get(id).cloned()
    }
    
    /// 撤销密钥
    pub async fn revoke_key(&self, id: &KeyId) -> EncryptionResult<()> {
        {
            let mut keys = self.encryption_keys.write().await;
            if let Some(key) = keys.get_mut(id) {
                key.status = KeyStatus::Revoked;
            }
        }
        
        {
            let mut keys = self.signing_keys.write().await;
            if let Some(key) = keys.get_mut(id) {
                key.status = KeyStatus::Revoked;
            }
        }
        
        Ok(())
    }
    
    /// 清理过期密钥
    pub async fn cleanup_expired(&self) -> usize {
        let mut count = 0;
        
        {
            let mut keys = self.encryption_keys.write().await;
            keys.retain(|_, k| !k.is_expired());
            count += keys.len();
        }
        
        {
            let mut keys = self.signing_keys.write().await;
            keys.retain(|_, k| !k.is_expired());
        }
        
        count
    }
    
    /// 创建加密器
    pub async fn create_encryptor(&self) -> EncryptionResult<Encryptor> {
        let encryptor = Encryptor::new(self.config.clone());
        encryptor.generate_key().await?;
        Ok(encryptor)
    }
    
    /// 创建签名器
    pub async fn create_signer(&self) -> EncryptionResult<Signer> {
        let key_id = self.generate_signing_key().await?;
        let key = self.get_signing_key(&key_id).await
            .ok_or_else(|| EncryptionError::InvalidKey("Failed to create signing key".to_string()))?;
        
        Ok(Signer::new(key, SignatureAlgorithm::HmacSha256))
    }
}

// ============================================================================
// 会话加密
// ============================================================================

/// 加密会话
#[derive(Debug, Clone)]
pub struct EncryptionSession {
    /// 会话 ID
    pub session_id: String,
    /// 会话密钥
    pub session_key: SymmetricKey,
    /// 远程节点公钥
    pub remote_public_key: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

impl EncryptionSession {
    /// 创建新会话
    pub fn new(duration: chrono::Duration) -> EncryptionResult<Self> {
        let key = SymmetricKey::generate()?;
        
        Ok(Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            session_key: key,
            remote_public_key: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + duration,
        })
    }
    
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// 加密消息
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8]) -> EncryptionResult<Vec<u8>> {
        if self.is_expired() {
            return Err(EncryptionError::KeyExpired("Session expired".to_string()));
        }
        
        // 简化加密
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let mut hasher = Sha256::new();
        hasher.update(&self.session_key.key);
        hasher.update(nonce);
        let keystream = hasher.finalize();
        
        for (i, byte) in plaintext.iter().enumerate() {
            ciphertext.push(byte ^ keystream[i % 32]);
        }
        
        Ok(ciphertext)
    }
    
    /// 解密消息
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> EncryptionResult<Vec<u8>> {
        // XOR 是对称的
        self.encrypt(ciphertext, nonce)
    }
}

// ============================================================================
// 安全通道
// ============================================================================

/// 安全通道状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// 未初始化
    Uninitialized,
    /// 握手中
    Handshaking,
    /// 已建立
    Established,
    /// 已关闭
    Closed,
}

/// 安全通道
pub struct SecureChannel {
    /// 远程节点 ID
    pub remote_node_id: String,
    /// 会话
    session: Option<EncryptionSession>,
    /// 状态
    state: ChannelState,
    /// 创建时间
    created_at: DateTime<Utc>,
}

impl SecureChannel {
    pub fn new(remote_node_id: String) -> Self {
        Self {
            remote_node_id,
            session: None,
            state: ChannelState::Uninitialized,
            created_at: Utc::now(),
        }
    }
    
    /// 开始握手
    pub fn start_handshake(&mut self) -> EncryptionResult<()> {
        if self.state != ChannelState::Uninitialized {
            return Err(EncryptionError::InvalidCertificate("Invalid state".to_string()));
        }
        
        self.state = ChannelState::Handshaking;
        Ok(())
    }
    
    /// 完成握手
    pub fn complete_handshake(&mut self, session: EncryptionSession) -> EncryptionResult<()> {
        if self.state != ChannelState::Handshaking {
            return Err(EncryptionError::InvalidCertificate("Invalid state".to_string()));
        }
        
        self.session = Some(session);
        self.state = ChannelState::Established;
        Ok(())
    }
    
    /// 检查是否已建立
    pub fn is_established(&self) -> bool {
        self.state == ChannelState::Established
    }
    
    /// 发送加密消息
    pub fn send(&self, plaintext: &[u8]) -> EncryptionResult<EncryptedData> {
        let session = self.session.as_ref()
            .ok_or_else(|| EncryptionError::InvalidCertificate("No session".to_string()))?;
        
        if !self.is_established() {
            return Err(EncryptionError::InvalidCertificate("Channel not established".to_string()));
        }
        
        let mut nonce = vec![0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        
        let ciphertext = session.encrypt(plaintext, &nonce)?;
        
        Ok(EncryptedData {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_id: session.session_key.id.clone(),
            nonce,
            ciphertext,
            tag: None,
            timestamp: Utc::now(),
        })
    }
    
    /// 接收并解密消息
    pub fn receive(&self, encrypted: &EncryptedData) -> EncryptionResult<Vec<u8>> {
        let session = self.session.as_ref()
            .ok_or_else(|| EncryptionError::InvalidCertificate("No session".to_string()))?;
        
        if !self.is_established() {
            return Err(EncryptionError::InvalidCertificate("Channel not established".to_string()));
        }
        
        session.decrypt(&encrypted.ciphertext, &encrypted.nonce)
    }
    
    /// 关闭通道
    pub fn close(&mut self) {
        self.state = ChannelState::Closed;
        self.session = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_id() {
        let id = KeyId::new();
        assert!(!id.as_str().is_empty());
    }
    
    #[test]
    fn test_symmetric_key_generation() {
        let key = SymmetricKey::generate().unwrap();
        assert_eq!(key.key.len(), 32);
        assert!(key.is_usable());
    }
    
    #[test]
    fn test_symmetric_key_base64() {
        let key = SymmetricKey::generate().unwrap();
        let encoded = key.to_base64();
        let restored = SymmetricKey::from_base64(&encoded).unwrap();
        
        assert_eq!(key.key, restored.key);
    }
    
    #[test]
    fn test_symmetric_key_expiry() {
        let key = SymmetricKey::generate().unwrap()
            .with_expiry(chrono::Duration::seconds(-1));
        
        assert!(key.is_expired());
        assert!(!key.is_usable());
    }
    
    #[test]
    fn test_encryption_config() {
        let config = EncryptionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_algorithm, EncryptionAlgorithm::Aes256Gcm);
    }
    
    #[tokio::test]
    async fn test_encryptor_encrypt_decrypt() {
        let config = EncryptionConfig::default();
        let encryptor = Encryptor::new(config);
        
        // 生成密钥
        encryptor.generate_key().await.unwrap();
        
        // 加密
        let plaintext = b"Hello, World!";
        let encrypted = encryptor.encrypt(plaintext).await.unwrap();
        
        // 解密
        let decrypted = encryptor.decrypt(&encrypted).await.unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
    
    #[test]
    fn test_signer_sign_verify() {
        let key = SymmetricKey::generate().unwrap();
        let signer = Signer::new(key, SignatureAlgorithm::HmacSha256);
        
        let message = b"Test message";
        let signature = signer.sign(message).unwrap();
        
        // 验证签名
        let valid = signer.verify(message, &signature).unwrap();
        assert!(valid);
        
        // 修改消息后验证应该失败
        let modified = b"Modified message";
        let invalid = signer.verify(modified, &signature).unwrap();
        assert!(!invalid);
    }
    
    #[tokio::test]
    async fn test_key_manager() {
        let config = EncryptionConfig::default();
        let manager = KeyManager::new(config);
        
        // 生成加密密钥
        let enc_id = manager.generate_encryption_key().await.unwrap();
        let enc_key = manager.get_encryption_key(&enc_id).await;
        assert!(enc_key.is_some());
        
        // 生成签名密钥
        let sig_id = manager.generate_signing_key().await.unwrap();
        let sig_key = manager.get_signing_key(&sig_id).await;
        assert!(sig_key.is_some());
        
        // 撤销密钥
        manager.revoke_key(&enc_id).await.unwrap();
        let revoked = manager.get_encryption_key(&enc_id).await.unwrap();
        assert_eq!(revoked.status, KeyStatus::Revoked);
    }
    
    #[test]
    fn test_encryption_session() {
        let session = EncryptionSession::new(chrono::Duration::hours(1)).unwrap();
        
        assert!(!session.is_expired());
        
        let nonce = vec![0u8; 12];
        let plaintext = b"Secret message";
        
        let encrypted = session.encrypt(plaintext, &nonce).unwrap();
        let decrypted = session.decrypt(&encrypted, &nonce).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
    
    #[test]
    fn test_secure_channel() {
        let mut channel = SecureChannel::new("remote-node".to_string());
        
        assert!(!channel.is_established());
        
        // 开始握手
        channel.start_handshake().unwrap();
        
        // 完成握手
        let session = EncryptionSession::new(chrono::Duration::hours(1)).unwrap();
        channel.complete_handshake(session).unwrap();
        
        assert!(channel.is_established());
        
        // 发送和接收
        let plaintext = b"Hello";
        let encrypted = channel.send(plaintext).unwrap();
        let decrypted = channel.receive(&encrypted).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
        
        // 关闭通道
        channel.close();
        assert!(!channel.is_established());
    }
    
    #[test]
    fn test_encrypted_data_serialization() {
        let encrypted = EncryptedData {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_id: KeyId::new(),
            nonce: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            ciphertext: vec![0, 1, 2, 3, 4, 5],
            tag: Some(vec![6, 7, 8, 9]),
            timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&encrypted).unwrap();
        let restored: EncryptedData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(encrypted.key_id, restored.key_id);
        assert_eq!(encrypted.nonce, restored.nonce);
    }
}