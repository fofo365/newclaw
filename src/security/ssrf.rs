// SSRF Protection - v0.5.1
//
// SSRF (Server-Side Request Forgery) 防护

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::Url;
use anyhow::{Result, anyhow};

/// SSRF 防护配置
#[derive(Debug, Clone)]
pub struct SsrfConfig {
    /// 允许的域名白名单
    pub allowed_domains: HashSet<String>,
    /// 禁止的 IP 地址
    pub denied_ips: HashSet<IpAddr>,
    /// 是否允许私有 IP
    pub allow_private_ip: bool,
    /// 是否允许本地回环地址
    pub allow_loopback: bool,
}

impl Default for SsrfConfig {
    fn default() -> Self {
        let mut denied_ips = HashSet::new();
        
        // 默认禁止的私有 IP 范围
        // 10.0.0.0/8
        for i in 0u8..=255 {
            denied_ips.insert(IpAddr::V4(Ipv4Addr::new(10, i, 0, 0)));
        }
        // 172.16.0.0/12
        for i in 16u8..32 {
            denied_ips.insert(IpAddr::V4(Ipv4Addr::new(172, i, 0, 0)));
        }
        // 192.168.0.0/16
        for i in 0u8..=255 {
            denied_ips.insert(IpAddr::V4(Ipv4Addr::new(192, 168, i, 0)));
        }
        // 127.0.0.0/8 (loopback)
        for i in 0u8..=255 {
            denied_ips.insert(IpAddr::V4(Ipv4Addr::new(127, i, 0, 1)));
        }
        
        Self {
            allowed_domains: HashSet::new(),
            denied_ips,
            allow_private_ip: false,
            allow_loopback: false,
        }
    }
}

impl SsrfConfig {
    /// 创建严格的 SSRF 配置
    pub fn strict() -> Self {
        Self {
            allow_private_ip: false,
            allow_loopback: false,
            ..Default::default()
        }
    }
    
    /// 创建宽松的 SSRF 配置（用于开发环境）
    pub fn permissive() -> Self {
        Self {
            allow_private_ip: true,
            allow_loopback: true,
            denied_ips: HashSet::new(),
            ..Default::default()
        }
    }
    
    /// 添加允许的域名
    pub fn allow_domain(&mut self, domain: &str) {
        self.allowed_domains.insert(domain.to_lowercase());
    }
    
    /// 添加禁止的 IP
    pub fn deny_ip(&mut self, ip: IpAddr) {
        self.denied_ips.insert(ip);
    }
}

/// SSRF 防护守卫
#[derive(Debug)]
pub struct SsrfGuard {
    config: SsrfConfig,
}

impl SsrfGuard {
    /// 创建新的 SSRF 守卫
    pub fn new(config: SsrfConfig) -> Self {
        Self { config }
    }
    
    /// 使用默认配置创建
    pub fn default_guard() -> Self {
        Self::new(SsrfConfig::default())
    }
    
    /// 验证 URL
    pub fn validate_url(&self, url: &str) -> Result<()> {
        let parsed = Url::parse(url)
            .map_err(|e| anyhow!("Invalid URL: {}", e))?;
        
        // 检查协议
        match parsed.scheme() {
            "http" | "https" => {},
            _ => return Err(anyhow!("Unsupported protocol: {}", parsed.scheme())),
        }
        
        // 检查域名
        if let Some(host) = parsed.host_str() {
            // 如果在白名单中，直接允许
            if self.config.allowed_domains.contains(&host.to_lowercase()) {
                return Ok(());
            }
            
            // 尝试解析为 IP
            if let Ok(ip) = host.parse::<IpAddr>() {
                self.validate_ip(&ip)?;
            } else {
                // 域名需要 DNS 解析，这里只做基础检查
                // 检查是否为 IP 格式的域名
                if host.parse::<Ipv4Addr>().is_ok() || host.parse::<Ipv6Addr>().is_ok() {
                    return Err(anyhow!("IP address not in whitelist: {}", host));
                }
            }
        }
        
        Ok(())
    }
    
    /// 验证 IP 地址
    pub fn validate_ip(&self, ip: &IpAddr) -> Result<()> {
        // 检查是否在禁止列表中
        if self.config.denied_ips.contains(ip) {
            return Err(anyhow!("IP address is denied: {}", ip));
        }
        
        // 检查私有 IP
        if !self.config.allow_private_ip && self.is_private_ip(ip) {
            return Err(anyhow!("Private IP address not allowed: {}", ip));
        }
        
        // 检查回环地址
        if !self.config.allow_loopback && self.is_loopback(ip) {
            return Err(anyhow!("Loopback address not allowed: {}", ip));
        }
        
        Ok(())
    }
    
    /// 检查是否为私有 IP
    fn is_private_ip(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_private() || ipv4.is_link_local()
            }
            IpAddr::V6(ipv6) => {
                // IPv6 私有地址检查
                ipv6.is_loopback() || 
                ipv6.to_string().starts_with("fc") || // Unique local
                ipv6.to_string().starts_with("fd") ||
                ipv6.to_string().starts_with("fe80") // Link local
            }
        }
    }
    
    /// 检查是否为回环地址
    fn is_loopback(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }
    
    /// 安全地获取 URL（返回验证后的 URL）
    pub fn safe_url(&self, url: &str) -> Result<String> {
        self.validate_url(url)?;
        Ok(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssrf_guard_valid_url() {
        let guard = SsrfGuard::default_guard();
        
        assert!(guard.validate_url("https://example.com").is_ok());
        assert!(guard.validate_url("http://example.com/path").is_ok());
    }

    #[test]
    fn test_ssrf_guard_private_ip() {
        let guard = SsrfGuard::default_guard();
        
        assert!(guard.validate_url("http://192.168.1.1").is_err());
        assert!(guard.validate_url("http://10.0.0.1").is_err());
        assert!(guard.validate_url("http://172.16.0.1").is_err());
    }

    #[test]
    fn test_ssrf_guard_loopback() {
        let guard = SsrfGuard::default_guard();
        
        assert!(guard.validate_url("http://127.0.0.1").is_err());
        // localhost 需要 DNS 解析，这里只测试 IP 格式
        assert!(guard.validate_url("http://127.0.0.1:8080").is_err());
    }

    #[test]
    fn test_ssrf_guard_whitelist() {
        let mut config = SsrfConfig::default();
        config.allow_domain("internal.example.com");
        config.allow_private_ip = true;
        
        let guard = SsrfGuard::new(config);
        
        // 白名单域名应该允许
        assert!(guard.validate_url("http://internal.example.com").is_ok());
    }

    #[test]
    fn test_ssrf_guard_invalid_protocol() {
        let guard = SsrfGuard::default_guard();
        
        assert!(guard.validate_url("ftp://example.com").is_err());
        assert!(guard.validate_url("file:///etc/passwd").is_err());
    }
}
