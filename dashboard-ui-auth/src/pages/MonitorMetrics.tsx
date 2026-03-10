import { useEffect, useState } from 'react'
import { Card, Row, Col, Statistic, Typography, Space, Tag } from 'antd'
import {
  ApiOutlined,
  CloudOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons'
import ReactECharts from 'echarts-for-react'
import { getMetrics, type MetricsResponse } from '../services/api'

const { Title, Text } = Typography

export default function MonitorMetrics() {
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadMetrics()
    const interval = setInterval(loadMetrics, 10000) // 每 10 秒刷新
    return () => clearInterval(interval)
  }, [])

  const loadMetrics = async () => {
    try {
      const res = await getMetrics()
      setMetrics(res.data)
    } catch (error) {
      console.error('Failed to load metrics:', error)
    } finally {
      setLoading(false)
    }
  }

  const formatUptime = (secs: number) => {
    const days = Math.floor(secs / 86400)
    const hours = Math.floor((secs % 86400) / 3600)
    const minutes = Math.floor((secs % 3600) / 60)
    if (days > 0) return `${days}天 ${hours}小时 ${minutes}分钟`
    if (hours > 0) return `${hours}小时 ${minutes}分钟`
    return `${minutes}分钟`
  }

  const latencyChartOption = {
    title: { text: '请求延迟分布', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: ['平均', 'P50', 'P95', 'P99'] },
    yAxis: { type: 'value', name: 'ms' },
    series: [{
      type: 'bar',
      data: metrics ? [
        { value: metrics.requests.avg_latency_ms, itemStyle: { color: '#1890ff' } },
        { value: metrics.requests.p50_latency_ms, itemStyle: { color: '#52c41a' } },
        { value: metrics.requests.p95_latency_ms, itemStyle: { color: '#faad14' } },
        { value: metrics.requests.p99_latency_ms, itemStyle: { color: '#ff4d4f' } },
      ] : [],
    }],
  }

  const requestChartOption = {
    title: { text: '请求统计', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'item' },
    legend: { bottom: 0 },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      avoidLabelOverlap: false,
      data: metrics ? [
        { value: metrics.requests.successful, name: '成功', itemStyle: { color: '#52c41a' } },
        { value: metrics.requests.failed, name: '失败', itemStyle: { color: '#ff4d4f' } },
      ] : [],
    }],
  }

  const tokenChartOption = {
    title: { text: 'Token 使用', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: ['输入', '输出'] },
    yAxis: { type: 'value', name: 'Token' },
    series: [{
      type: 'bar',
      data: metrics ? [
        { value: metrics.tokens.total_input, itemStyle: { color: '#1890ff' } },
        { value: metrics.tokens.total_output, itemStyle: { color: '#722ed1' } },
      ] : [],
    }],
  }

  return (
    <div>
      <div className="page-header">
        <Title level={2}>性能指标</Title>
        <Text type="secondary">实时监控系统性能</Text>
      </div>

      {/* 连接状态 */}
      <Row gutter={[16, 16]}>
        <Col span={24}>
          <Card loading={loading}>
            <Space size="large">
              <div>
                <Text type="secondary">运行时间</Text>
                <br />
                <Tag icon={<ClockCircleOutlined />} color="blue" style={{ fontSize: 14 }}>
                  {metrics ? formatUptime(metrics.uptime_secs) : '-'}
                </Tag>
              </div>
              <div>
                <Text type="secondary">飞书 WebSocket</Text>
                <br />
                {metrics?.connections.feishu_websocket ? (
                  <Tag icon={<CheckCircleOutlined />} color="success">已连接</Tag>
                ) : (
                  <Tag icon={<CloseCircleOutlined />} color="warning">未连接</Tag>
                )}
              </div>
              <div>
                <Text type="secondary">活跃会话</Text>
                <br />
                <Tag color="blue">{metrics?.connections.active_sessions || 0}</Tag>
              </div>
              <div>
                <Text type="secondary">活跃 WebSocket</Text>
                <br />
                <Tag color="blue">{metrics?.connections.active_websockets || 0}</Tag>
              </div>
            </Space>
          </Card>
        </Col>
      </Row>

      {/* 请求指标 */}
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
              title="成功请求"
              value={metrics?.requests.successful || 0}
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="失败请求"
              value={metrics?.requests.failed || 0}
              valueStyle={{ color: '#cf1322' }}
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

      {/* Token 指标 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} sm={8}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="输入 Token"
              value={metrics?.tokens.total_input || 0}
              prefix={<CloudOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="输出 Token"
              value={metrics?.tokens.total_output || 0}
              prefix={<CloudOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card className="stat-card" loading={loading}>
            <Statistic
              title="Token/分钟"
              value={(metrics?.tokens.rate_per_minute || 0).toFixed(1)}
            />
          </Card>
        </Col>
      </Row>

      {/* 图表 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={8}>
          <Card loading={loading}>
            <ReactECharts option={latencyChartOption} style={{ height: 250 }} />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card loading={loading}>
            <ReactECharts option={requestChartOption} style={{ height: 250 }} />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card loading={loading}>
            <ReactECharts option={tokenChartOption} style={{ height: 250 }} />
          </Card>
        </Col>
      </Row>

      {/* 延迟详情 */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="延迟详情" loading={loading}>
            <Row gutter={16}>
              <Col span={6}>
                <Statistic title="平均延迟" value={metrics?.requests.avg_latency_ms || 0} suffix="ms" />
              </Col>
              <Col span={6}>
                <Statistic title="P50 延迟" value={metrics?.requests.p50_latency_ms || 0} suffix="ms" />
              </Col>
              <Col span={6}>
                <Statistic title="P95 延迟" value={metrics?.requests.p95_latency_ms || 0} suffix="ms" />
              </Col>
              <Col span={6}>
                <Statistic title="P99 延迟" value={metrics?.requests.p99_latency_ms || 0} suffix="ms" />
              </Col>
            </Row>
          </Card>
        </Col>
      </Row>

      {/* 最后错误 */}
      {metrics?.errors.last_error && (
        <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
          <Col span={24}>
            <Card title="最近错误" loading={loading}>
              <Space direction="vertical" style={{ width: '100%' }}>
                <Text type="danger">{metrics.errors.last_error}</Text>
                <Text type="secondary">
                  发生时间: {metrics.errors.last_error_time}
                </Text>
              </Space>
            </Card>
          </Col>
        </Row>
      )}
    </div>
  )
}
