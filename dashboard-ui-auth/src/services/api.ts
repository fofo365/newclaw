import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json',
  },
})

// 响应拦截器
api.interceptors.response.use(
  (response) => response,
  (error) => {
    console.error('API Error:', error)
    return Promise.reject(error)
  }
)

// ============== LLM 配置 ==============

export interface LLMConfig {
  provider: string
  model: string
  temperature: number
  max_tokens: number
  providers: ProviderInfo[]
}

export interface ProviderInfo {
  name: string
  display_name: string
  configured: boolean
  models: string[]
}

export const getLLMConfig = () => api.get<LLMConfig>('/config/llm')
export const updateLLMConfig = (data: Partial<LLMConfig>) => api.put('/config/llm', data)

// ============== 工具配置 ==============

export interface ToolInfo {
  name: string
  display_name: string
  description: string
  enabled: boolean
  category: string
  parameters: ToolParameter[]
}

export interface ToolParameter {
  name: string
  type: string
  description: string
  required: boolean
  default?: string
}

export const getToolsConfig = () => api.get<{ tools: ToolInfo[] }>('/config/tools')
export const updateToolsConfig = (data: { enabled_tools: string[] }) => api.put('/config/tools', data)

// ============== 飞书配置 ==============

export interface FeishuConfig {
  configured: boolean
  app_id?: string
  app_secret?: string
  encrypt_key?: string
  verification_token?: string
  connection_mode: string
  webhook_url?: string
  events_enabled: boolean
}

export const getFeishuConfig = () => api.get<FeishuConfig>('/config/feishu')
export const updateFeishuConfig = (data: Partial<FeishuConfig>) => api.put('/config/feishu', data)

// ============== 监控 ==============

export interface LogEntry {
  id: string
  timestamp: string
  level: string
  source: string
  message: string
  metadata: Record<string, unknown>
}

export interface LogsResponse {
  logs: LogEntry[]
  total: number
  has_more: boolean
}

export const getLogs = (params?: {
  level?: string
  source?: string
  search?: string
  limit?: number
  offset?: number
}) => api.get<LogsResponse>('/monitor/logs', { params })

export interface MetricsResponse {
  uptime_secs: number
  requests: {
    total: number
    successful: number
    failed: number
    avg_latency_ms: number
    p50_latency_ms: number
    p95_latency_ms: number
    p99_latency_ms: number
  }
  tokens: {
    total_input: number
    total_output: number
    total: number
    rate_per_minute: number
  }
  connections: {
    feishu_websocket: boolean
    active_sessions: number
    active_websockets: number
  }
  errors: {
    total_errors: number
    error_rate: number
    last_error?: string
    last_error_time?: string
  }
}

export const getMetrics = () => api.get<MetricsResponse>('/monitor/metrics')
export const getHealth = () => api.get('/monitor/health')

// ============== 对话 ==============

export interface ChatSession {
  id: string
  title: string
  created_at: string
  updated_at: string
  messages: ChatMessage[]
  metadata: Record<string, unknown>
}

export interface ChatMessage {
  id: string
  role: string
  content: string
  timestamp: string
  tokens?: {
    input: number
    output: number
    total: number
  }
  metadata: Record<string, unknown>
}

export const listSessions = () => api.get<{ sessions: ChatSession[], total: number }>('/chat/sessions')
export const createSession = (data: { title?: string }) => api.post<ChatSession>('/chat/sessions', data)
export const getSession = (id: string) => api.get<ChatSession>(`/chat/sessions/${id}`)
export const sendMessage = (sessionId: string, content: string) =>
  api.post<ChatMessage>(`/chat/sessions/${sessionId}/messages`, { content })

// ============== 管理 ==============

export interface UserInfo {
  id: string
  username: string
  email?: string
  role: string
  created_at: string
  last_login?: string
  is_active: boolean
  quota: {
    max_requests_per_day?: number
    max_tokens_per_day?: number
    used_requests: number
    used_tokens: number
  }
}

export const listUsers = () => api.get<{ users: UserInfo[], total: number }>('/admin/users')
export const createUser = (data: { username: string; password: string; email?: string; role?: string }) =>
  api.post<UserInfo>('/admin/users', data)
export const deleteUser = (id: string) => api.delete(`/admin/users/${id}`)

export interface ApiKeyInfo {
  id: string
  name: string
  prefix: string
  created_at: string
  expires_at?: string
  last_used?: string
  is_active: boolean
  permissions: string[]
  usage: {
    total_requests: number
    total_tokens: number
    last_24h_requests: number
    last_24h_tokens: number
  }
}

export const listApiKeys = () => api.get<ApiKeyInfo[]>('/admin/apikeys')
export const createApiKey = (data: { name: string; expires_in_days?: number; permissions?: string[] }) =>
  api.post<{ info: ApiKeyInfo; key: string }>('/admin/apikeys', data)
export const revokeApiKey = (id: string) => api.delete(`/admin/apikeys/${id}`)

export default api
