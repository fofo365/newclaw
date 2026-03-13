// NewClaw v0.4.0 - 企业微信（WeCom）消息加密/解密
//
// 实现企业微信消息的 AES-256-CBC 加密/解密和签名验证

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha1::{Digest, Sha1};

use super::types::crypto::*;

/// 解码 EncodingAESKey
///
/// 将企业微信配置的 Base64 编码的 AES Key 解码为字节。
/// 包含补全 Padding 和长度校验（必须 32 字节）。
pub fn decode_encoding_aes_key(encoding_aes_key: &str) -> Result<Vec<u8>> {
    let trimmed = encoding_aes_key.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("encodingAESKey missing"));
    }
    
    // 补全 Base64 Padding
    let with_padding = if trimmed.ends_with('=') {
        trimmed.to_string()
    } else {
        format!("{}=", trimmed)
    };
    
    let key = BASE64.decode(&with_padding)
        .map_err(|e| anyhow!("invalid encodingAESKey base64: {}", e))?;
    
    if key.len() != AES_KEY_LENGTH {
        return Err(anyhow!(
            "invalid encodingAESKey (expected {} bytes, got {})",
            AES_KEY_LENGTH,
            key.len()
        ));
    }
    
    Ok(key)
}

/// PKCS#7 填充
pub fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let mod_val = data.len() % block_size;
    let pad = if mod_val == 0 { block_size } else { block_size - mod_val };
    let mut result = data.to_vec();
    result.extend(std::iter::repeat_n(pad as u8, pad));
    result
}

/// PKCS#7 去填充
pub fn pkcs7_unpad(data: &[u8], block_size: usize) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Err(anyhow!("invalid pkcs7 payload: empty"));
    }
    
    let pad = data[data.len() - 1] as usize;
    if pad < 1 || pad > block_size {
        return Err(anyhow!("invalid pkcs7 padding: {}", pad));
    }
    if pad > data.len() {
        return Err(anyhow!("invalid pkcs7 payload: pad {} > len {}", pad, data.len()));
    }
    
    // 验证所有填充字节
    for i in 0..pad {
        if data[data.len() - 1 - i] as usize != pad {
            return Err(anyhow!("invalid pkcs7 padding: inconsistent"));
        }
    }
    
    Ok(data[..data.len() - pad].to_vec())
}

/// 计算消息签名
///
/// 算法：sha1(sort(token, timestamp, nonce, encrypt_msg))
pub fn compute_msg_signature(token: &str, timestamp: &str, nonce: &str, encrypt: &str) -> String {
    let mut parts = [token, timestamp, nonce, encrypt];
    parts.sort();
    
    let mut hasher = Sha1::new();
    hasher.update(parts.join("").as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 验证消息签名
pub fn verify_signature(token: &str, timestamp: &str, nonce: &str, encrypt: &str, signature: &str) -> bool {
    let expected = compute_msg_signature(token, timestamp, nonce, encrypt);
    expected == signature
}

/// AES-256-CBC 解密
fn aes_cbc_decrypt(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    // 使用 ring 库进行 AES 解密
    let key_len = key.len();
    if key_len != 32 {
        return Err(anyhow!("AES key must be 32 bytes, got {}", key_len));
    }
    if iv.len() != 16 {
        return Err(anyhow!("IV must be 16 bytes, got {}", iv.len()));
    }
    
    // 简单的 AES CBC 解密实现（使用 aes 库的底层 API）
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockDecrypt, KeyInit};
    
    type Aes256 = aes::Aes256;
    
    let cipher = Aes256::new(GenericArray::from_slice(key));
    
    let mut result = ciphertext.to_vec();
    let mut prev_block = iv.to_vec();
    
    for chunk in result.chunks_mut(16) {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        
        // 保存当前密文块
        let current_cipher = block;
        
        // 解密块
        cipher.decrypt_block(GenericArray::from_mut_slice(&mut block));
        
        // XOR 与前一个密文块（或 IV）
        for i in 0..16 {
            chunk[i] = block[i] ^ prev_block[i];
        }
        
        // 更新前一个块
        prev_block = current_cipher.to_vec();
    }
    
    Ok(result)
}

/// AES-256-CBC 加密
fn aes_cbc_encrypt(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockEncrypt, KeyInit};
    
    type Aes256 = aes::Aes256;
    
    let cipher = Aes256::new(GenericArray::from_slice(key));
    
    let mut result = plaintext.to_vec();
    let mut prev_block = iv.to_vec();
    
    for chunk in result.chunks_mut(16) {
        // 补齐最后一个块
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        
        // XOR 与前一个密文块（或 IV）
        for i in 0..16 {
            block[i] ^= prev_block[i];
        }
        
        // 加密块
        cipher.encrypt_block(GenericArray::from_mut_slice(&mut block));
        
        // 更新结果和前一个块
        chunk.copy_from_slice(&block);
        prev_block = block.to_vec();
    }
    
    Ok(result)
}

/// 解密企业微信消息
///
/// 流程：
/// 1. Base64 解码 AESKey 并获取 IV（前 16 字节）
/// 2. AES-CBC 解密
/// 3. 去除 PKCS#7 填充
/// 4. 拆解协议包结构：[16 字节随机串][4 字节长度][消息体][接收者 ID]
/// 5. 校验接收者 ID（ReceiveId）
pub fn decrypt_message(encoding_aes_key: &str, encrypt: &str, receive_id: Option<&str>) -> Result<String> {
    let aes_key = decode_encoding_aes_key(encoding_aes_key)?;
    let iv = &aes_key[..IV_LENGTH];
    
    // Base64 解码加密内容
    let encrypted = BASE64.decode(encrypt)
        .map_err(|e| anyhow!("invalid encrypt base64: {}", e))?;
    
    // AES-CBC 解密
    let decrypted_padded = aes_cbc_decrypt(&aes_key, iv, &encrypted)?;
    
    // 去除 PKCS#7 填充
    let decrypted = pkcs7_unpad(&decrypted_padded, PKCS7_BLOCK_SIZE)?;
    
    if decrypted.len() < 20 {
        return Err(anyhow!("invalid decrypted payload (expected at least 20 bytes, got {})", decrypted.len()));
    }
    
    // 解析消息结构：[16 bytes random][4 bytes length][msg][receiveId]
    let msg_len = u32::from_be_bytes([decrypted[16], decrypted[17], decrypted[18], decrypted[19]]) as usize;
    let msg_start = 20;
    let msg_end = msg_start + msg_len;
    
    if msg_end > decrypted.len() {
        return Err(anyhow!("invalid decrypted msg length (msgEnd={}, payloadLength={})", msg_end, decrypted.len()));
    }
    
    let msg = String::from_utf8(decrypted[msg_start..msg_end].to_vec())
        .map_err(|e| anyhow!("invalid utf8 message: {}", e))?;
    
    // 验证 receive_id
    if let Some(expected_id) = receive_id {
        if !expected_id.is_empty() {
            let trailing = String::from_utf8_lossy(&decrypted[msg_end..]);
            if trailing != expected_id {
                return Err(anyhow!("receiveId mismatch (expected '{}', got '{}')", expected_id, trailing));
            }
        }
    }
    
    Ok(msg)
}

/// 加密企业微信消息
///
/// 流程：
/// 1. 构造协议包：[16 字节随机串][4 字节长度][消息体][接收者 ID]
/// 2. PKCS#7 填充
/// 3. AES-CBC 加密
/// 4. 转 Base64
pub fn encrypt_message(encoding_aes_key: &str, plaintext: &str, receive_id: Option<&str>) -> Result<String> {
    let aes_key = decode_encoding_aes_key(encoding_aes_key)?;
    let iv = &aes_key[..IV_LENGTH];
    
    // 构造消息结构
    let mut random16 = [0u8; 16];
    getrandom::getrandom(&mut random16)
        .map_err(|e| anyhow!("random fill failed: {}", e))?;
    
    let msg = plaintext.as_bytes();
    let msg_len = (msg.len() as u32).to_be_bytes();
    let receive_id_bytes = receive_id.unwrap_or("").as_bytes();
    
    let mut raw = Vec::with_capacity(16 + 4 + msg.len() + receive_id_bytes.len());
    raw.extend_from_slice(&random16);
    raw.extend_from_slice(&msg_len);
    raw.extend_from_slice(msg);
    raw.extend_from_slice(receive_id_bytes);
    
    // PKCS#7 填充
    let padded = pkcs7_pad(&raw, PKCS7_BLOCK_SIZE);
    
    // AES-CBC 加密
    let encrypted = aes_cbc_encrypt(&aes_key, iv, &padded)?;
    
    Ok(BASE64.encode(&encrypted))
}

/// WeCom 加密客户端
pub struct WeComCrypto {
    encoding_aes_key: String,
    token: Option<String>,
    receive_id: Option<String>,
}

impl WeComCrypto {
    pub fn new(encoding_aes_key: String, token: Option<String>, receive_id: Option<String>) -> Result<Self> {
        // 验证 key 格式
        decode_encoding_aes_key(&encoding_aes_key)?;
        Ok(Self {
            encoding_aes_key,
            token,
            receive_id,
        })
    }
    
    /// 解密消息
    pub fn decrypt(&self, encrypt: &str) -> Result<String> {
        decrypt_message(&self.encoding_aes_key, encrypt, self.receive_id.as_deref())
    }
    
    /// 加密消息
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        encrypt_message(&self.encoding_aes_key, plaintext, self.receive_id.as_deref())
    }
    
    /// 验证签名
    pub fn verify(&self, timestamp: &str, nonce: &str, encrypt: &str, signature: &str) -> bool {
        if let Some(token) = &self.token {
            verify_signature(token, timestamp, nonce, encrypt, signature)
        } else {
            false
        }
    }
    
    /// 计算签名
    pub fn compute_signature(&self, timestamp: &str, nonce: &str, encrypt: &str) -> Option<String> {
        self.token.as_ref().map(|token| {
            compute_msg_signature(token, timestamp, nonce, encrypt)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pkcs7_pad_unpad() {
        let data = b"hello";
        let padded = pkcs7_pad(data, 32);
        assert_eq!(padded.len() % 32, 0);
        
        let unpadded = pkcs7_unpad(&padded, 32).unwrap();
        assert_eq!(unpadded.as_slice(), data);
    }
    
    #[test]
    fn test_compute_msg_signature() {
        let sig = compute_msg_signature("token", "1234567890", "nonce", "encrypt");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 40); // SHA1 hex
    }
    
    #[test]
    fn test_verify_signature() {
        let token = "test_token";
        let timestamp = "1234567890";
        let nonce = "abc123";
        let encrypt = "encrypted_content";
        
        let signature = compute_msg_signature(token, timestamp, nonce, encrypt);
        assert!(verify_signature(token, timestamp, nonce, encrypt, &signature));
        assert!(!verify_signature(token, timestamp, nonce, encrypt, "wrong_signature"));
    }
    
    #[test]
    fn test_aes_cbc_encrypt_decrypt() {
        // 测试加密解密往返
        let key = [1u8; 32];
        let iv = [2u8; 16];
        let plaintext = b"Hello, World! This is a test message.";
        
        // 填充到 32 字节块
        let padded = pkcs7_pad(plaintext, 32);
        
        let encrypted = aes_cbc_encrypt(&key, &iv, &padded).unwrap();
        let decrypted = aes_cbc_decrypt(&key, &iv, &encrypted).unwrap();
        let unpadded = pkcs7_unpad(&decrypted, 32).unwrap();
        
        assert_eq!(unpadded.as_slice(), plaintext);
    }
}
