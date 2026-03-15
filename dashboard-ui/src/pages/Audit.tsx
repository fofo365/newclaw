import { useEffect, useState } from 'react'
import { Card, Table, Button, Tag, Space, DatePicker, Select, Form, Row, Col, Statistic, message, Typography } from 'antd'
import {
  SearchOutlined,
  DownloadOutlined,
  SecurityScanOutlined,
  WarningOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'
import dayjs from 'dayjs'

const { Title, Text } = Typography
const { RangePicker } = DatePicker

interface AuditLog {
  id: string
  event_type: string
  user_id?: string
  action: string
  resource?: string
  ip_address?: string
  status: string
  created_at: string
  details: string
}

interface AuditStats {
  total_events: number
  failed_logins: number
  security_alerts: number
}

const eventTypeColor: Record<string, string> = {
  login: 'blue',
  logout: 'default',
  api_call: 'cyan',
  config_change: 'orange',
  tool_execution: 'purple',
  memory_access: 'geekblue',
  admin_action: 'red',
  security_alert: 'magenta',
}

const statusColor: Record<string, string> = {
  success: 'success',
  failed: 'error',
  denied: 'warning',
}

export default function Audit() {
  const [logs, setLogs] = useState<AuditLog[]>([])
  const [stats, setStats] = useState<AuditStats>({ total_events: 0, failed_logins: 0, security_alerts: 0 })
  const [loading, setLoading] = useState(true)
  const [form] = Form.useForm()

  const fetchData = async (params?: any) => {
    setLoading(true)
    try {
      const query = new URLSearchParams()
      if (params?.event_type) query.set('event_type', params.event_type)
      if (params?.status) query.set('status', params.status)
      if (params?.start_time) query.set('start_time', params.start_time)
      if (params?.end_time) query.set('end_time', params.end_time)
      query.set('limit', '100')
      
      const res = await fetch(`/api/audit/logs?${query.toString()}`)
      if (res.ok) {
        const data = await res.json()
        setLogs(data.logs || [])
      }
      
      const statsRes = await fetch('/api/audit/stats')
      if (statsRes.ok) {
        const data = await statsRes.json()
        setStats(data)
      }
    } catch (error) {
      console.error('Failed to fetch audit logs:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
  }, [])

  const handleSearch = (values: any) => {
    const params: any = {}
    if (values.event_type) params.event_type = values.event_type
    if (values.status) params.status = values.status
    if (values.time_range) {
      params.start_time = values.time_range[0].toISOString()
      params.end_time = values.time_range[1].toISOString()
    }
    fetchData(params)
  }

  const handleExport = async () => {
    try {
      const res = await fetch('/api/audit/export')
      if (res.ok) {
        const data = await res.json()
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `audit_logs_${dayjs().format('YYYYMMDD_HHmmss')}.json`
        a.click()
        URL.revokeObjectURL(url)
        message.success('导出成功')
      }
    } catch (error) {
      message.error('导出失败')
    }
  }

  const columns: ColumnsType<AuditLog> = [
    {
      title: '时间',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 180,
      render: (t: string) => new Date(t).toLocaleString(),
    },
    {
      title: '事件类型',
      dataIndex: 'event_type',
      key: 'event_type',
      width: 120,
      render: (type: string) => (
        <Tag color={eventTypeColor[type] || 'default'}>
          {type?.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: '操作',
      dataIndex: 'action',
      key: 'action',
      ellipsis: true,
    },
    {
      title: '用户',
      dataIndex: 'user_id',
      key: 'user_id',
      width: 120,
      render: (id?: string) => id || '-',
    },
    {
      title: '资源',
      dataIndex: 'resource',
      key: 'resource',
      width: 150,
      ellipsis: true,
      render: (r?: string) => r || '-',
    },
    {
      title: 'IP',
      dataIndex: 'ip_address',
      key: 'ip_address',
      width: 130,
      render: (ip?: string) => ip || '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 80,
      render: (status: string) => (
        <Tag color={statusColor[status] || 'default'}>
          {status?.toUpperCase()}
        </Tag>
      ),
    },
  ]

  return (
    <div>
      <div className="page-header">
        <Title level={2}>审计日志</Title>
        <Text type="secondary">安全事件记录与操作追踪</Text>
      </div>

      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="总事件数"
              value={stats.total_events}
              prefix={<SecurityScanOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="失败登录"
              value={stats.failed_logins}
              prefix={<WarningOutlined />}
              valueStyle={{ color: stats.failed_logins > 0 ? '#cf1322' : '#3f8600' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="安全告警"
              value={stats.security_alerts}
              prefix={<WarningOutlined />}
              valueStyle={{ color: stats.security_alerts > 0 ? '#cf1322' : '#3f8600' }}
            />
          </Card>
        </Col>
      </Row>

      {/* 筛选表单 */}
      <Card style={{ marginBottom: 16 }}>
        <Form form={form} layout="inline" onFinish={handleSearch}>
          <Form.Item name="event_type" label="事件类型">
            <Select allowClear placeholder="全部" style={{ width: 150 }}>
              <Select.Option value="login">登录</Select.Option>
              <Select.Option value="logout">登出</Select.Option>
              <Select.Option value="api_call">API调用</Select.Option>
              <Select.Option value="config_change">配置变更</Select.Option>
              <Select.Option value="tool_execution">工具执行</Select.Option>
              <Select.Option value="admin_action">管理操作</Select.Option>
              <Select.Option value="security_alert">安全告警</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear placeholder="全部" style={{ width: 120 }}>
              <Select.Option value="success">成功</Select.Option>
              <Select.Option value="failed">失败</Select.Option>
              <Select.Option value="denied">拒绝</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="time_range" label="时间范围">
            <RangePicker showTime />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" icon={<SearchOutlined />}>
                查询
              </Button>
              <Button icon={<DownloadOutlined />} onClick={handleExport}>
                导出
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      {/* 日志表格 */}
      <Card loading={loading}>
        <Table
          columns={columns}
          dataSource={logs}
          rowKey="id"
          pagination={{ pageSize: 20, showSizeChanger: true, showTotal: (total) => `共 ${total} 条` }}
        />
      </Card>
    </div>
  )
}