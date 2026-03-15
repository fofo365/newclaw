import { useEffect, useState } from 'react'
import { Card, Row, Col, Statistic, Table, Tag, Space, Button, Typography } from 'antd'
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
  WarningOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'

const { Title, Text } = Typography

interface HealthStatus {
  component: string
  status: string
  last_check: string
  message: string
}

interface LeaseInfo {
  id: string
  holder: string
  expires_at: string
  status: string
}

interface RecoveryAction {
  id: string
  type: string
  status: string
  created_at: string
  details: string
}

export default function Watchdog() {
  const [healthStatus, setHealthStatus] = useState<HealthStatus[]>([])
  const [leases, setLeases] = useState<LeaseInfo[]>([])
  const [recoveryActions, setRecoveryActions] = useState<RecoveryAction[]>([])
  const [loading, setLoading] = useState(true)

  const fetchData = async () => {
    setLoading(true)
    try {
      // 模拟数据 - 实际应从 API 获取
      setHealthStatus([
        { component: 'LLM Provider', status: 'healthy', last_check: new Date().toISOString(), message: 'OK' },
        { component: 'Database', status: 'healthy', last_check: new Date().toISOString(), message: 'Connected' },
        { component: 'Memory Store', status: 'healthy', last_check: new Date().toISOString(), message: 'OK' },
        { component: 'Feishu Connection', status: 'warning', last_check: new Date().toISOString(), message: 'Not configured' },
        { component: 'Redis Cache', status: 'healthy', last_check: new Date().toISOString(), message: 'Connected' },
      ])
      setLeases([
        { id: 'lease-001', holder: 'gateway-main', expires_at: new Date(Date.now() + 30000).toISOString(), status: 'active' },
      ])
      setRecoveryActions([])
    } catch (error) {
      console.error('Failed to fetch watchdog data:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchData, 10000)
    return () => clearInterval(interval)
  }, [])

  const healthColumns: ColumnsType<HealthStatus> = [
    { title: '组件', dataIndex: 'component', key: 'component' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: string) => (
        <Tag color={status === 'healthy' ? 'success' : status === 'warning' ? 'warning' : 'error'}>
          {status === 'healthy' ? <CheckCircleOutlined /> : status === 'warning' ? <WarningOutlined /> : <CloseCircleOutlined />}
          {' '}{status.toUpperCase()}
        </Tag>
      ),
    },
    { title: '消息', dataIndex: 'message', key: 'message' },
    {
      title: '最后检查',
      dataIndex: 'last_check',
      key: 'last_check',
      render: (t: string) => new Date(t).toLocaleTimeString(),
    },
  ]

  const leaseColumns: ColumnsType<LeaseInfo> = [
    { title: 'Lease ID', dataIndex: 'id', key: 'id' },
    { title: '持有者', dataIndex: 'holder', key: 'holder' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: string) => (
        <Tag color={status === 'active' ? 'success' : 'default'}>{status}</Tag>
      ),
    },
    {
      title: '过期时间',
      dataIndex: 'expires_at',
      key: 'expires_at',
      render: (t: string) => new Date(t).toLocaleTimeString(),
    },
  ]

  const recoveryColumns: ColumnsType<RecoveryAction> = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '类型', dataIndex: 'type', key: 'type' },
    { title: '状态', dataIndex: 'status', key: 'status' },
    { title: '创建时间', dataIndex: 'created_at', key: 'created_at' },
    { title: '详情', dataIndex: 'details', key: 'details' },
  ]

  const healthyCount = healthStatus.filter(h => h.status === 'healthy').length
  const warningCount = healthStatus.filter(h => h.status === 'warning').length
  const errorCount = healthStatus.filter(h => h.status === 'error').length

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title level={2}>Watchdog 监控</Title>
          <Text type="secondary">系统健康监控与故障恢复</Text>
        </div>
        <Space>
          <Button icon={<SyncOutlined />} onClick={fetchData}>刷新</Button>
        </Space>
      </div>

      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="健康组件"
              value={healthyCount}
              prefix={<CheckCircleOutlined style={{ color: '#52c41a' }} />}
              valueStyle={{ color: '#52c41a' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="警告组件"
              value={warningCount}
              prefix={<WarningOutlined style={{ color: '#faad14' }} />}
              valueStyle={{ color: '#faad14' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="故障组件"
              value={errorCount}
              prefix={<CloseCircleOutlined style={{ color: '#ff4d4f' }} />}
              valueStyle={{ color: '#ff4d4f' }}
            />
          </Card>
        </Col>
      </Row>

      {/* 健康状态 */}
      <Card title="组件健康状态" style={{ marginBottom: 16 }} loading={loading}>
        <Table
          columns={healthColumns}
          dataSource={healthStatus}
          rowKey="component"
          pagination={false}
          size="small"
        />
      </Card>

      {/* Lease 信息 */}
      <Card title="Lease 管理" style={{ marginBottom: 16 }} loading={loading}>
        <Table
          columns={leaseColumns}
          dataSource={leases}
          rowKey="id"
          pagination={false}
          size="small"
          locale={{ emptyText: '暂无活跃 Lease' }}
        />
      </Card>

      {/* 恢复操作 */}
      <Card title="恢复操作记录" loading={loading}>
        <Table
          columns={recoveryColumns}
          dataSource={recoveryActions}
          rowKey="id"
          pagination={false}
          size="small"
          locale={{ emptyText: '暂无恢复操作记录' }}
        />
      </Card>
    </div>
  )
}