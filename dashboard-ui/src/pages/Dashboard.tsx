import { useEffect, useState } from 'react'
import { Card, Row, Col, Statistic, Progress, Tag, Typography, Space } from 'antd'
import {
  CloudOutlined,
  MessageOutlined,
  ApiOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons'
import ReactECharts from 'echarts-for-react'
import { getHealth, getMetrics, type MetricsResponse } from '../services/api'

const { Title, Text } = Typography

export default function Dashboard() {
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [health, setHealth] = useState<{
    status: string
    components: {
      llm: { status: string; message?: string }
      feishu: { status: string; message?: string }
      database: { status: string; message?: string }
    }
  } | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [metricsRes, healthRes] = await Promise.all([
          getMetrics(),
          getHealth(),
        ])
        setMetrics(metricsRes.data)
        setHealth(healthRes.data as typeof health)
      } catch (error) {
        console.error('Failed to fetch dashboard data:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
    const interval = setInterval(fetchData, 30000) // 每 30 秒刷新
    return () => clearInterval(interval)
  }, [])

  const formatUptime = (secs: number) => {
    const days = Math.floor(secs / 86400)
    const hours = Math.floor((secs % 86400) / 3600)
    const minutes = Math.floor((secs % 3600) / 60)
    if (days > 0) return `${days}d ${hours}h ${minutes}m`
    if (hours > 0) return `${hours}h ${minutes}m`
    return `${minutes}m`
  }

  const latencyChartOption = {
    title: { text: '请求延迟分布', left: 'center' },
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: ['P50', 'P95', 'P99', 'Avg'] },
    yAxis: { type: 'value', name: 'ms' },
    series: [{
      type: 'bar',
      data: metrics ? [
        metrics.requests.p50_latency_ms,
        metrics.requests.p95_latency_ms,
        metrics.requests.p99_latency_ms,
        metrics.requests.avg_latency_ms,
      ] : [0, 0, 0, 0],
      itemStyle: { color: '#1890ff' },
    }],
  }

  const tokenChartOption = {
    title: { text: 'Token 使用统计', left: 'center' },
    tooltip: { trigger: 'item' },
    legend: { orient: 'vertical', left: 'left' },
    series: [{
      type: 'pie',
      radius: '50%',
      data: metrics ? [
        { value: metrics.tokens.total_input, name: '输入 Token' },
        { value: metrics.tokens.total_output, name: '输出 Token' },
      ] : [],
      emphasis: {
        itemStyle: {
          shadowBlur: 10,
          shadowOffsetX: 0,
          shadowColor: 'rgba(0, 0, 0, 0.5)',
        },
      },
    }],
  }

  return (
    <div>
      <div className="page-header">
        <Title level={2}>仪表盘</Title>
        <Text type="secondary">系统运行状态概览</Text>
      </div>

      {/* 组件状态 */}
      <Row gutter={[16, 16]}>
        <Col span={24}>
          <Card title="组件状态" loading={loading}>
            <Space size="large">
              <div>
                <Text type="secondary">LLM Provider</Text>
                <br />
                {health?.components.llm.status === 'ok' ? (
                  <Tag icon={<CheckCircleOutlined />} color="success">正常</Tag>
                ) : (
                  <Tag icon={<CloseCircleOutlined />} color="warning">{health?.components.llm.message || '异常'}</Tag>
                )}
              </div>
              <div>
                <Text type="secondary">飞书连接</Text>
                <br />
                {health?.components.feishu.status === 'ok' ? (
                  <Tag icon={<CheckCircleOutlined />} color="success">已连接</Tag>
                ) : (
                  <Tag icon={<CloseCircleOutlined />} color="warning">未连接</Tag>
                )}
              </div>
              <div>
                <Text type="secondary">数据库</Text>
                <br />
                {health?.components.database.status === 'ok' ? (
                  <Tag icon={<CheckCircleOutlined />} color="success">正常</Tag>
                ) : (
                  <Tag icon={<CloseCircleOutlined />} color="error">异常</Tag>
                )}
              </div>
              <div>
                <Text type="secondary">运行时间</Text>
                <br />
                <Tag icon={<ClockCircleOutlined />} color="blue">{metrics ? formatUptime(metrics.uptime_secs) : '-'}</Tag>
              </div>
            </Space>
          </Card>
        </Col>
      </Row>

      {/* 关键指标 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} sm={12} md={6}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="总请求数"
              value={metrics?.requests.total || 0}
              prefix={<ApiOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="活跃会话"
              value={metrics?.connections.active_sessions || 0}
              prefix={<MessageOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="Token 总量"
              value={metrics?.tokens.total || 0}
              prefix={<CloudOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="错误率"
              value={((metrics?.errors.error_rate || 0) * 100).toFixed(2)}
              suffix="%"
              valueStyle={{ color: (metrics?.errors.error_rate || 0) > 0.01 ? '#cf1322' : '#3f8600' }}
            />
          </Card>
        </Col>
      </Row>

      {/* 图表 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={12}>
          <Card loading={loading}>
            <ReactECharts option={latencyChartOption} style={{ height: 300 }} />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card loading={loading}>
            <ReactECharts option={tokenChartOption} style={{ height: 300 }} />
          </Card>
        </Col>
      </Row>

      {/* 请求成功率 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="请求成功率" loading={loading}>
            <Progress
              percent={metrics ? Math.round((metrics.requests.successful / Math.max(metrics.requests.total, 1)) * 100) : 0}
              status={metrics && metrics.requests.failed > 0 ? 'exception' : 'success'}
            />
            <div style={{ marginTop: 16 }}>
              <Text type="secondary">
                成功: {metrics?.requests.successful || 0} / 失败: {metrics?.requests.failed || 0}
              </Text>
            </div>
          </Card>
        </Col>
      </Row>
    </div>
  )
}
