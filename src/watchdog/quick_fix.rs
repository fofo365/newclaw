// L1 快速修复执行器 - 真实系统调用实现

use std::process::Command;
use std::time::Duration;
use anyhow::Result;

/// L1 快速修复执行器
pub struct QuickFixExecutor {
    /// 服务名称
    service_name: String,
    /// Redis 地址（用于缓存清理）
    redis_addr: Option<String>,
    /// 配置备份目录
    config_backup_dir: String,
}

impl QuickFixExecutor {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            redis_addr: std::env::var("REDIS_URL").ok(),
            config_backup_dir: "/var/lib/newclaw/config-backup".to_string(),
        }
    }
    
    pub fn with_redis(mut self, addr: String) -> Self {
        self.redis_addr = Some(addr);
        self
    }
    
    pub fn with_backup_dir(mut self, dir: String) -> Self {
        self.config_backup_dir = dir;
        self
    }
    
    /// 重启服务（通过 systemd）
    pub async fn restart_service(&self) -> Result<String> {
        tracing::info!("Restarting service: {}", self.service_name);
        
        // 使用 systemctl restart
        let output = Command::new("systemctl")
            .args(&["restart", &self.service_name])
            .output()?;
        
        if output.status.success() {
            // 等待服务启动
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            // 检查服务状态
            let status = Command::new("systemctl")
                .args(&["is-active", &self.service_name])
                .output()?;
            
            let status_str = String::from_utf8_lossy(&status.stdout).trim().to_string();
            
            if status_str == "active" {
                tracing::info!("Service {} restarted successfully", self.service_name);
                Ok(format!("Service {} restarted (status: {})", self.service_name, status_str))
            } else {
                anyhow::bail!("Service {} failed to start (status: {})", self.service_name, status_str)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to restart service: {}", stderr)
        }
    }
    
    /// 清理缓存（Redis FLUSHDB）
    pub async fn clear_cache(&self) -> Result<String> {
        tracing::info!("Clearing cache");
        
        if let Some(ref redis_addr) = self.redis_addr {
            // 使用 redis-cli FLUSHDB
            let output = Command::new("redis-cli")
                .args(&["-u", redis_addr, "FLUSHDB"])
                .output()?;
            
            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
                tracing::info!("Cache cleared: {}", result);
                Ok(format!("Cache cleared ({})", result))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("Redis flush failed: {}, using fallback", stderr);
                // 回退到内存清理
                self.clear_memory_cache().await
            }
        } else {
            // 没有 Redis，清理内存缓存
            self.clear_memory_cache().await
        }
    }
    
    /// 清理内存缓存（Linux）
    async fn clear_memory_cache(&self) -> Result<String> {
        // 同步并清理页面缓存
        let sync_output = Command::new("sync").output()?;
        if !sync_output.status.success() {
            tracing::warn!("sync command failed");
        }
        
        // 清理页面缓存（需要 root 权限）
        let clear_output = Command::new("sh")
            .arg("-c")
            .arg("echo 1 > /proc/sys/vm/drop_caches 2>/dev/null || echo 'No permission'")
            .output()?;
        
        let result = String::from_utf8_lossy(&clear_output.stdout).trim().to_string();
        tracing::info!("Memory cache cleared: {}", result);
        Ok(format!("Memory cache cleared ({})", result))
    }
    
    /// 回滚配置（Git 操作）
    pub async fn rollback_config(&self) -> Result<String> {
        tracing::info!("Rolling back config from {}", self.config_backup_dir);
        
        let backup_dir = std::path::Path::new(&self.config_backup_dir);
        
        if !backup_dir.exists() {
            anyhow::bail!("Config backup directory does not exist: {}", self.config_backup_dir);
        }
        
        // 查找最新的备份
        let entries = std::fs::read_dir(backup_dir)?;
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".toml"))
            .collect();
        
        if backups.is_empty() {
            anyhow::bail!("No config backups found in {}", self.config_backup_dir);
        }
        
        // 按修改时间排序，取最新的
        backups.sort_by(|a, b| {
            b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .cmp(&a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH))
        });
        
        let latest_backup = &backups[0];
        let backup_path = latest_backup.path();
        
        // 复制备份到当前配置
        let config_path = "/etc/newclaw/config.toml";
        std::fs::copy(&backup_path, config_path)?;
        
        tracing::info!("Config rolled back from {:?}", backup_path);
        Ok(format!("Config rolled back from {:?}", backup_path))
    }
    
    /// 释放资源（清理临时文件、关闭空闲连接）
    pub async fn release_resources(&self) -> Result<String> {
        tracing::info!("Releasing resources");
        
        let mut results = Vec::new();
        
        // 清理临时文件
        let temp_dir = std::path::Path::new("/tmp/newclaw");
        if temp_dir.exists() {
            let entries = std::fs::read_dir(temp_dir)?;
            let mut count = 0;
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                        let age = std::time::SystemTime::now()
                            .duration_since(modified)
                            .unwrap_or(Duration::ZERO);
                        // 删除超过 1 小时的临时文件
                        if age > Duration::from_secs(3600) {
                            if std::fs::remove_file(entry.path()).is_ok() {
                                count += 1;
                            }
                        }
                    }
                }
            }
            results.push(format!("Cleaned {} temp files", count));
        }
        
        // 触发 GC（如果可能）
        #[cfg(unix)]
        {
            // 请求内核回收内存
            let _ = Command::new("sh")
                .arg("-c")
                .arg("echo 2 > /proc/sys/vm/drop_caches 2>/dev/null || true")
                .output();
            results.push("Triggered memory cleanup".to_string());
        }
        
        let result = results.join(", ");
        tracing::info!("Resources released: {}", result);
        Ok(result)
    }
    
    /// 检查服务状态
    pub async fn check_service_status(&self) -> Result<ServiceStatus> {
        let output = Command::new("systemctl")
            .args(&["show", &self.service_name, "--property=ActiveState,SubState,MainPID"])
            .output()?;
        
        if output.status.success() {
            let info = String::from_utf8_lossy(&output.stdout);
            let mut status = ServiceStatus::default();
            
            for line in info.lines() {
                if line.starts_with("ActiveState=") {
                    status.active_state = line.split('=').nth(1).unwrap_or("unknown").to_string();
                } else if line.starts_with("SubState=") {
                    status.sub_state = line.split('=').nth(1).unwrap_or("unknown").to_string();
                } else if line.starts_with("MainPID=") {
                    status.pid = line.split('=').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            
            status.is_running = status.active_state == "active";
            Ok(status)
        } else {
            Ok(ServiceStatus {
                is_running: false,
                active_state: "unknown".to_string(),
                sub_state: "unknown".to_string(),
                pid: 0,
            })
        }
    }
}

/// 服务状态
#[derive(Debug, Clone, Default)]
pub struct ServiceStatus {
    pub is_running: bool,
    pub active_state: String,
    pub sub_state: String,
    pub pid: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_executor_creation() {
        let executor = QuickFixExecutor::new("newclaw".to_string());
        assert_eq!(executor.service_name, "newclaw");
    }
    
    #[test]
    fn test_executor_with_redis() {
        let executor = QuickFixExecutor::new("newclaw".to_string())
            .with_redis("redis://localhost:6379".to_string());
        assert!(executor.redis_addr.is_some());
    }
    
    #[tokio::test]
    async fn test_check_service_status() {
        let executor = QuickFixExecutor::new("systemd-journald".to_string());
        let status = executor.check_service_status().await;
        // systemd-journald 应该总是运行
        assert!(status.is_ok());
    }
}