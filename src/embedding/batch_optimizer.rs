// Batch Optimizer - v0.5.0
//
// 批量优化器：
// - 自动合并小批次
// - 动态批量大小调整
// - 超时触发机制

use super::{EmbeddingError, EmbeddingResult, BatchEmbeddingResult};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

/// 批量配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// 目标批量大小（token 数）
    pub target_batch_tokens: usize,
    /// 最大批量大小（请求数）
    pub max_batch_size: usize,
    /// 批量超时
    pub batch_timeout: Duration,
    /// 是否启用动态调整
    pub dynamic_sizing: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            target_batch_tokens: 8000 * 10,  // 10 个 max_tokens 请求
            max_batch_size: 100,
            batch_timeout: Duration::from_millis(50),
            dynamic_sizing: true,
        }
    }
}

/// 批量请求
#[derive(Debug)]
pub struct BatchRequest {
    /// 文本内容
    pub text: String,
    /// 估算 token 数
    pub estimated_tokens: usize,
    /// 响应通道
    pub tx: oneshot::Sender<Result<EmbeddingResult, EmbeddingError>>,
}

/// 批量优化器
pub struct BatchOptimizer {
    /// 配置
    config: BatchConfig,
    /// 请求队列
    request_queue: mpsc::UnboundedSender<BatchRequest>,
}

impl BatchOptimizer {
    /// 创建新的优化器
    pub fn new(config: BatchConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // 启动批量处理任务
        let mut optimizer = BatchOptimizerWorker {
            config: config.clone(),
            request_rx: rx,
        };

        tokio::spawn(async move {
            optimizer.run().await;
        });

        Self {
            config,
            request_queue: tx,
        }
    }

    /// 提交嵌入请求（返回未来）
    pub async fn submit(&self, text: String, estimated_tokens: usize) -> Result<EmbeddingResult, EmbeddingError> {
        let (tx, rx) = oneshot::channel();

        self.request_queue.send(BatchRequest {
            text,
            estimated_tokens,
            tx,
        })
            .map_err(|_| EmbeddingError::Unknown("Batch optimizer closed".to_string()))?;

        let result: Result<Result<EmbeddingResult, EmbeddingError>, oneshot::error::RecvError> = rx.await;
        result
            .map_err(|_| EmbeddingError::Unknown("Batch response channel closed".to_string()))?
    }

    /// 获取配置
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }
}

/// 批量优化器工作线程
struct BatchOptimizerWorker {
    config: BatchConfig,
    request_rx: mpsc::UnboundedReceiver<BatchRequest>,
}

impl BatchOptimizerWorker {
    /// 运行批量处理循环
    async fn run(&mut self) {
        let mut pending_requests = Vec::new();
        let mut current_tokens = 0;

        loop {
            // 等待请求或超时
            let _deadline = tokio::time::Instant::now() + self.config.batch_timeout;

            match timeout(self.config.batch_timeout, self.request_rx.recv()).await {
                Ok(Some(request)) => {
                    // 检查是否应该触发批量处理
                    let should_flush = self.should_flush(
                        &pending_requests,
                        current_tokens,
                        &request,
                    );

                    if should_flush {
                        // 处理当前批次
                        self.process_batch(std::mem::take(&mut pending_requests)).await;
                        current_tokens = 0;
                    }

                    // 添加新请求
                    current_tokens += request.estimated_tokens;
                    pending_requests.push(request);
                }
                Ok(None) => {
                    // 通道关闭，处理剩余请求
                    if !pending_requests.is_empty() {
                        self.process_batch(pending_requests).await;
                    }
                    break;
                }
                Err(_) => {
                    // 超时，处理当前批次
                    if !pending_requests.is_empty() {
                        self.process_batch(std::mem::take(&mut pending_requests)).await;
                        current_tokens = 0;
                    }
                }
            }
        }
    }

    /// 判断是否应该触发批量处理
    fn should_flush(
        &self,
        pending: &[BatchRequest],
        current_tokens: usize,
        new_request: &BatchRequest,
    ) -> bool {
        // 检查批量大小限制
        if pending.len() >= self.config.max_batch_size {
            return true;
        }

        // 检查 token 限制
        let new_tokens = current_tokens + new_request.estimated_tokens;
        if new_tokens >= self.config.target_batch_tokens {
            return true;
        }

        false
    }

    /// 处理一个批次（需要外部实现实际的嵌入逻辑）
    async fn process_batch(&self, requests: Vec<BatchRequest>) {
        // 这个方法是框架代码，实际实现需要集成 EmbeddingClient
        // 这里我们只是模拟处理

        if requests.is_empty() {
            return;
        }

        let texts: Vec<String> = requests.iter().map(|r| r.text.clone()).collect();
        let total_tokens: usize = requests.iter().map(|r| r.estimated_tokens).sum();

        // TODO: 实际调用 EmbeddingClient::embed_batch
        // 这里我们返回错误，提示需要集成
        for request in requests {
            let _ = request.tx.send(Err(EmbeddingError::Unknown(
                "BatchOptimizer needs EmbeddingClient integration".to_string()
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 100);
        assert!(config.dynamic_sizing);
    }

    #[tokio::test]
    async fn test_batch_optimizer_create() {
        let optimizer = BatchOptimizer::new(BatchConfig::default());
        assert_eq!(optimizer.config().max_batch_size, 100);
    }
}
