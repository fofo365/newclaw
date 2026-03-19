// DAG Workflow Engine - 有向无环图工作流引擎
//
// v0.7.0 - 实现任务依赖编排和拓扑排序执行

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use tokio::sync::RwLock;

/// DAG 节点 ID
pub type NodeId = String;

/// DAG 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// 节点 ID
    pub id: NodeId,
    /// 节点名称
    pub name: String,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub params: serde_json::Value,
    /// 前置节点 ID 列表
    pub predecessors: HashSet<NodeId>,
    /// 后继节点 ID 列表
    pub successors: HashSet<NodeId>,
    /// 节点状态
    pub state: DagNodeState,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行结果
    pub result: Option<DagNodeResult>,
}

/// DAG 节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DagNodeState {
    /// 待执行
    Pending,
    /// 就绪（前置条件满足）
    Ready,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 跳过
    Skipped,
    /// 取消
    Cancelled,
}

/// DAG 节点执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNodeResult {
    /// 是否成功
    pub success: bool,
    /// 输出数据
    pub output: serde_json::Value,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
}

/// DAG 工作流
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagWorkflow {
    /// 工作流 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 节点映射
    pub nodes: HashMap<NodeId, DagNode>,
    /// 入口节点（无前置依赖）
    pub entry_nodes: Vec<NodeId>,
    /// 出口节点（无后继节点）
    pub exit_nodes: Vec<NodeId>,
    /// 工作流状态
    pub state: DagWorkflowState,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// DAG 工作流状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DagWorkflowState {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 部分成功
    PartialSuccess,
    /// 失败
    Failed,
    /// 取消
    Cancelled,
}

/// DAG 构建器
pub struct DagBuilder {
    nodes: HashMap<NodeId, DagNode>,
    name: String,
}

impl DagBuilder {
    /// 创建新的 DAG 构建器
    pub fn new(name: &str) -> Self {
        Self {
            nodes: HashMap::new(),
            name: name.to_string(),
        }
    }
    
    /// 添加节点
    pub fn add_node(mut self, id: &str, name: &str, task_type: &str) -> Self {
        let node = DagNode {
            id: id.to_string(),
            name: name.to_string(),
            task_type: task_type.to_string(),
            params: serde_json::Value::Null,
            predecessors: HashSet::new(),
            successors: HashSet::new(),
            state: DagNodeState::Pending,
            retry_count: 0,
            max_retries: 3,
            timeout_secs: 300,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
        };
        self.nodes.insert(id.to_string(), node);
        self
    }
    
    /// 设置节点参数
    pub fn with_params(mut self, id: &str, params: serde_json::Value) -> Self {
        if let Some(node) = self.nodes.get_mut(id) {
            node.params = params;
        }
        self
    }
    
    /// 添加依赖关系（edge: from -> to，to 依赖 from）
    pub fn add_edge(mut self, from: &str, to: &str) -> Result<Self> {
        // 检查节点是否存在
        if !self.nodes.contains_key(from) {
            anyhow::bail!("Source node '{}' not found", from);
        }
        if !self.nodes.contains_key(to) {
            anyhow::bail!("Target node '{}' not found", to);
        }
        
        // 添加依赖关系
        self.nodes.get_mut(from).unwrap().successors.insert(to.to_string());
        self.nodes.get_mut(to).unwrap().predecessors.insert(from.to_string());
        
        // 检查是否形成环
        if self.has_cycle() {
            anyhow::bail!("Adding edge {} -> {} would create a cycle", from, to);
        }
        
        Ok(self)
    }
    
    /// 设置重试次数
    pub fn with_retry(mut self, id: &str, max_retries: u32) -> Self {
        if let Some(node) = self.nodes.get_mut(id) {
            node.max_retries = max_retries;
        }
        self
    }
    
    /// 设置超时
    pub fn with_timeout(mut self, id: &str, timeout_secs: u64) -> Self {
        if let Some(node) = self.nodes.get_mut(id) {
            node.timeout_secs = timeout_secs;
        }
        self
    }
    
    /// 检查是否有环
    fn has_cycle(&self) -> bool {
        // 使用 DFS 检测环
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for node_id in self.nodes.keys() {
            if self.dfs_cycle_check(node_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        
        false
    }
    
    fn dfs_cycle_check(
        &self,
        node_id: &str,
        visited: &mut HashSet<NodeId>,
        rec_stack: &mut HashSet<NodeId>,
    ) -> bool {
        if rec_stack.contains(node_id) {
            return true;
        }
        
        if visited.contains(node_id) {
            return false;
        }
        
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());
        
        if let Some(node) = self.nodes.get(node_id) {
            for successor in &node.successors {
                if self.dfs_cycle_check(successor, visited, rec_stack) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(node_id);
        false
    }
    
    /// 构建 DAG 工作流
    pub fn build(self) -> Result<DagWorkflow> {
        // 找出入口节点（无前置依赖）
        let entry_nodes: Vec<NodeId> = self.nodes.values()
            .filter(|n| n.predecessors.is_empty())
            .map(|n| n.id.clone())
            .collect();
        
        if entry_nodes.is_empty() && !self.nodes.is_empty() {
            anyhow::bail!("DAG has no entry nodes (all nodes have predecessors - possible cycle)");
        }
        
        // 找出出口节点（无后继节点）
        let exit_nodes: Vec<NodeId> = self.nodes.values()
            .filter(|n| n.successors.is_empty())
            .map(|n| n.id.clone())
            .collect();
        
        Ok(DagWorkflow {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name,
            nodes: self.nodes,
            entry_nodes,
            exit_nodes,
            state: DagWorkflowState::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        })
    }
}

/// DAG 执行器
pub struct DagExecutor {
    /// 执行器配置
    config: DagExecutorConfig,
}

/// DAG 执行器配置
#[derive(Debug, Clone)]
pub struct DagExecutorConfig {
    /// 最大并发数
    pub max_concurrency: usize,
    /// 默认超时（秒）
    pub default_timeout_secs: u64,
    /// 失败时是否继续执行
    pub continue_on_failure: bool,
}

impl Default for DagExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            default_timeout_secs: 300,
            continue_on_failure: false,
        }
    }
}

/// DAG 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagExecutionResult {
    /// 工作流 ID
    pub workflow_id: String,
    /// 是否成功
    pub success: bool,
    /// 成功节点数
    pub success_count: usize,
    /// 失败节点数
    pub failed_count: usize,
    /// 跳过节点数
    pub skipped_count: usize,
    /// 总节点数
    pub total_count: usize,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 节点结果
    pub node_results: HashMap<NodeId, DagNodeResult>,
}

impl DagExecutor {
    /// 创建新的执行器
    pub fn new(config: DagExecutorConfig) -> Self {
        Self { config }
    }
    
    /// 执行 DAG 工作流（拓扑排序）
    pub async fn execute<F, Fut>(&self, workflow: &mut DagWorkflow, mut executor: F) -> Result<DagExecutionResult>
    where
        F: FnMut(NodeId, String, serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = Result<DagNodeResult>>,
    {
        let start_time = Utc::now();
        workflow.state = DagWorkflowState::Running;
        workflow.started_at = Some(start_time);
        
        let mut results: HashMap<NodeId, DagNodeResult> = HashMap::new();
        let mut completed: HashSet<NodeId> = HashSet::new();
        let mut ready_queue: VecDeque<NodeId> = workflow.entry_nodes.iter().cloned().collect();
        
        // 标记入口节点为就绪
        for node_id in &workflow.entry_nodes {
            if let Some(node) = workflow.nodes.get_mut(node_id) {
                node.state = DagNodeState::Ready;
            }
        }
        
        while let Some(node_id) = ready_queue.pop_front() {
            let node = workflow.nodes.get(&node_id).cloned();
            
            if let Some(node) = node {
                // 执行节点
                let result = self.execute_node(&node, &mut executor).await;
                
                // 更新节点状态
                let workflow_node = workflow.nodes.get_mut(&node_id).unwrap();
                workflow_node.result = Some(result.clone());
                workflow_node.completed_at = Some(Utc::now());
                
                if result.success {
                    workflow_node.state = DagNodeState::Success;
                    completed.insert(node_id.clone());
                    results.insert(node_id.clone(), result);
                    
                    // 更新后继节点的就绪状态
                    for successor_id in &workflow_node.successors.clone() {
                        if let Some(successor) = workflow.nodes.get_mut(successor_id) {
                            // 检查所有前置节点是否完成
                            let all_predecessors_done = successor.predecessors.iter()
                                .all(|pred| completed.contains(pred));
                            
                            if all_predecessors_done {
                                successor.state = DagNodeState::Ready;
                                ready_queue.push_back(successor_id.clone());
                            }
                        }
                    }
                } else {
                    workflow_node.state = DagNodeState::Failed;
                    results.insert(node_id.clone(), result);
                    
                    if !self.config.continue_on_failure {
                        // 标记剩余节点为跳过
                        for (id, node) in workflow.nodes.iter_mut() {
                            if node.state == DagNodeState::Pending || node.state == DagNodeState::Ready {
                                node.state = DagNodeState::Skipped;
                            }
                        }
                        break;
                    }
                }
            }
        }
        
        // 计算结果
        let end_time = Utc::now();
        workflow.completed_at = Some(end_time);
        
        let success_count = workflow.nodes.values()
            .filter(|n| n.state == DagNodeState::Success)
            .count();
        let failed_count = workflow.nodes.values()
            .filter(|n| n.state == DagNodeState::Failed)
            .count();
        let skipped_count = workflow.nodes.values()
            .filter(|n| n.state == DagNodeState::Skipped)
            .count();
        
        let success = failed_count == 0 && skipped_count == 0;
        workflow.state = if success {
            DagWorkflowState::Success
        } else if success_count > 0 {
            DagWorkflowState::PartialSuccess
        } else {
            DagWorkflowState::Failed
        };
        
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;
        
        Ok(DagExecutionResult {
            workflow_id: workflow.id.clone(),
            success,
            success_count,
            failed_count,
            skipped_count,
            total_count: workflow.nodes.len(),
            duration_ms,
            node_results: results,
        })
    }
    
    /// 执行单个节点
    async fn execute_node<F, Fut>(&self, node: &DagNode, executor: &mut F) -> DagNodeResult
    where
        F: FnMut(NodeId, String, serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = Result<DagNodeResult>>,
    {
        let start = Utc::now();
        
        let result = executor(node.id.clone(), node.task_type.clone(), node.params.clone()).await;
        
        let end = Utc::now();
        let duration_ms = (end - start).num_milliseconds() as u64;
        
        match result {
            Ok(mut r) => {
                r.duration_ms = duration_ms;
                r
            }
            Err(e) => DagNodeResult {
                success: false,
                output: serde_json::Value::Null,
                error: Some(e.to_string()),
                duration_ms,
            },
        }
    }
    
    /// 获取拓扑排序
    pub fn topological_sort(workflow: &DagWorkflow) -> Result<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut sorted: Vec<NodeId> = Vec::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        
        // 初始化入度
        for (id, node) in &workflow.nodes {
            in_degree.insert(id.clone(), node.predecessors.len());
        }
        
        // 入度为 0 的节点入队
        for (id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(id.clone());
            }
        }
        
        // 拓扑排序
        while let Some(node_id) = queue.pop_front() {
            sorted.push(node_id.clone());
            
            if let Some(node) = workflow.nodes.get(&node_id) {
                for successor in &node.successors {
                    if let Some(degree) = in_degree.get_mut(successor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(successor.clone());
                        }
                    }
                }
            }
        }
        
        if sorted.len() != workflow.nodes.len() {
            anyhow::bail!("DAG contains a cycle - topological sort failed");
        }
        
        Ok(sorted)
    }
}

/// DAG 检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagCheckpoint {
    /// 检查点 ID
    pub id: String,
    /// 工作流 ID
    pub workflow_id: String,
    /// 已完成的节点
    pub completed_nodes: HashSet<NodeId>,
    /// 节点结果
    pub node_results: HashMap<NodeId, DagNodeResult>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl DagCheckpoint {
    /// 创建检查点
    pub fn create(workflow: &DagWorkflow) -> Self {
        let completed_nodes: HashSet<NodeId> = workflow.nodes.values()
            .filter(|n| n.state == DagNodeState::Success)
            .map(|n| n.id.clone())
            .collect();
        
        let node_results: HashMap<NodeId, DagNodeResult> = workflow.nodes.values()
            .filter(|n| n.result.is_some())
            .map(|n| (n.id.clone(), n.result.clone().unwrap()))
            .collect();
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id: workflow.id.clone(),
            completed_nodes,
            node_results,
            created_at: Utc::now(),
        }
    }
    
    /// 从检查点恢复
    pub fn restore(&self, workflow: &mut DagWorkflow) {
        for node_id in &self.completed_nodes {
            if let Some(node) = workflow.nodes.get_mut(node_id) {
                node.state = DagNodeState::Success;
                if let Some(result) = self.node_results.get(node_id) {
                    node.result = Some(result.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dag_builder_simple() {
        let dag = DagBuilder::new("test-dag")
            .add_node("a", "Task A", "shell")
            .add_node("b", "Task B", "shell")
            .add_edge("a", "b").unwrap()
            .build().unwrap();
        
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.entry_nodes, vec!["a"]);
        assert_eq!(dag.exit_nodes, vec!["b"]);
    }
    
    #[test]
    fn test_dag_builder_complex() {
        let dag = DagBuilder::new("complex-dag")
            .add_node("a", "Task A", "shell")
            .add_node("b", "Task B", "shell")
            .add_node("c", "Task C", "shell")
            .add_node("d", "Task D", "shell")
            .add_edge("a", "b").unwrap()
            .add_edge("a", "c").unwrap()
            .add_edge("b", "d").unwrap()
            .add_edge("c", "d").unwrap()
            .build().unwrap();
        
        assert_eq!(dag.nodes.len(), 4);
        assert_eq!(dag.entry_nodes, vec!["a"]);
        assert_eq!(dag.exit_nodes, vec!["d"]);
    }
    
    #[test]
    fn test_dag_builder_cycle_detection() {
        let result = DagBuilder::new("cycle-dag")
            .add_node("a", "Task A", "shell")
            .add_node("b", "Task B", "shell")
            .add_edge("a", "b").unwrap()
            .add_edge("b", "a");
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_topological_sort() {
        let dag = DagBuilder::new("sort-test")
            .add_node("a", "A", "shell")
            .add_node("b", "B", "shell")
            .add_node("c", "C", "shell")
            .add_edge("a", "b").unwrap()
            .add_edge("b", "c").unwrap()
            .build().unwrap();
        
        let sorted = DagExecutor::topological_sort(&dag).unwrap();
        
        // a 必须在 b 之前，b 必须在 c 之前
        let a_pos = sorted.iter().position(|x| x == "a").unwrap();
        let b_pos = sorted.iter().position(|x| x == "b").unwrap();
        let c_pos = sorted.iter().position(|x| x == "c").unwrap();
        
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }
    
    #[tokio::test]
    async fn test_dag_executor() {
        let mut dag = DagBuilder::new("exec-test")
            .add_node("a", "Task A", "echo")
            .add_node("b", "Task B", "echo")
            .add_edge("a", "b").unwrap()
            .build().unwrap();
        
        let executor = DagExecutor::new(DagExecutorConfig::default());
        
        let result = executor.execute(&mut dag, |id, task_type, _params| {
            async move {
                Ok(DagNodeResult {
                    success: true,
                    output: serde_json::json!({ "id": id, "type": task_type }),
                    error: None,
                    duration_ms: 100,
                })
            }
        }).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failed_count, 0);
    }
    
    #[test]
    fn test_dag_checkpoint() {
        let mut dag = DagBuilder::new("checkpoint-test")
            .add_node("a", "A", "shell")
            .add_node("b", "B", "shell")
            .build().unwrap();
        
        // 模拟完成节点 a
        dag.nodes.get_mut("a").unwrap().state = DagNodeState::Success;
        dag.nodes.get_mut("a").unwrap().result = Some(DagNodeResult {
            success: true,
            output: serde_json::Value::Null,
            error: None,
            duration_ms: 100,
        });
        
        let checkpoint = DagCheckpoint::create(&dag);
        
        assert!(checkpoint.completed_nodes.contains("a"));
        assert!(!checkpoint.completed_nodes.contains("b"));
    }
    
    #[tokio::test]
    async fn test_dag_executor_with_failure() {
        let mut dag = DagBuilder::new("failure-test")
            .add_node("a", "A", "fail")
            .add_node("b", "B", "echo")
            .add_edge("a", "b").unwrap()
            .build().unwrap();
        
        let executor = DagExecutor::new(DagExecutorConfig::default());
        
        let result = executor.execute(&mut dag, |id, _task_type, _params| {
            async move {
                if id == "a" {
                    Ok(DagNodeResult {
                        success: false,
                        output: serde_json::Value::Null,
                        error: Some("Failed".to_string()),
                        duration_ms: 100,
                    })
                } else {
                    Ok(DagNodeResult {
                        success: true,
                        output: serde_json::Value::Null,
                        error: None,
                        duration_ms: 100,
                    })
                }
            }
        }).await.unwrap();
        
        assert!(!result.success);
        assert_eq!(result.failed_count, 1);
        assert_eq!(result.skipped_count, 1); // b 被跳过
    }
}