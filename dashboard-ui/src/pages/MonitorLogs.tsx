import { useEffect, useState } from 'react'
import { Card, Input, Select, Button, Space, Tag, Typography, Table, message } from 'antd'
import { ReloadOutlined, DownloadOutlined } from '@ant-design/icons'
import { getLogs, type LogEntry } from '../services/api'
import dayjs from 'dayjs'

const { Title, Text } = Typography
const { Option } = Select

export default function MonitorLogs() {
  const [loading, setLoading] = useState(false)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [total, setTotal] = useState(0)
  const [level, setLevel] = useState<string>('')
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const pageSize = 50

  useEffect(() => {
    loadLogs()
  }, [page, level])

  const loadLogs = async () => {
    setLoading(true)
    try {
      const res = await getLogs({
        level: level || undefined,
        search: search || undefined,
        limit: pageSize,
        offset: (page - 1) * pageSize,
      })
      setLogs(res.data.logs)
      setTotal(res.data.total)
    } catch (error) {
      message.error('加载日志失败')
    } finally {
      setLoading(false)
    }
  }

  const handleSearch = () => {
    setPage(1)
    loadLogs()
  }

  const getLevelColor = (lvl: string) => {
    switch (lvl.toLowerCase()) {
      case 'error': return 'red'
      case 'warn': return 'orange'
      case 'info': return 'blue'
      case 'debug': return 'green'
      case 'trace': return 'purple'
      default: return 'default'
    }
  }

  const exportLogs = () => {
    const content = logs.map(log => 
      `[${log.timestamp}] [${log.level.toUpperCase()}] [${log.source}] ${log.message}`
    ).join('\n')
    
    const blob = new Blob([content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `newclaw-logs-${dayjs().format('YYYY-MM-DD-HHmmss')}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  const columns = [
    {
      title: '时间',
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 180,
      render: (ts: string) => dayjs(ts).format('MM-DD HH:mm:ss.SSS'),
    },
    {
      title: '级别',
      dataIndex: 'level',
      key: 'level',
      width: 80,
      render: (level: string) => (
        <Tag color={getLevelColor(level)}>{level.toUpperCase()}</Tag>
      ),
    },
    {
      title: '来源',
      dataIndex: 'source',
      key: 'source',
      width: 150,
      render: (source: string) => <Text code>{source}</Text>,
    },
    {
      title: '消息',
      dataIndex: 'message',
      key: 'message',
      ellipsis: true,
    },
  ]

  return (
    <div>
      <div className="page-header">
        <Title level={2}>日志查看</Title>
        <Text type="secondary">实时查看系统日志</Text>
      </div>

      <Card>
        {/* 过滤器 */}
        <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
          <Space>
            <Select
              placeholder="日志级别"
              style={{ width: 120 }}
              allowClear
              value={level || undefined}
              onChange={(v) => setLevel(v || '')}
            >
              <Option value="error">Error</Option>
              <Option value="warn">Warn</Option>
              <Option value="info">Info</Option>
              <Option value="debug">Debug</Option>
              <Option value="trace">Trace</Option>
            </Select>
            <Input.Search
              placeholder="搜索日志内容"
              style={{ width: 250 }}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onSearch={handleSearch}
              enterButton
            />
          </Space>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={loadLogs}>
              刷新
            </Button>
            <Button icon={<DownloadOutlined />} onClick={exportLogs}>
              导出
            </Button>
          </Space>
        </Space>

        {/* 日志表格 */}
        <Table
          dataSource={logs}
          columns={columns}
          rowKey="id"
          loading={loading}
          size="small"
          pagination={{
            current: page,
            pageSize,
            total,
            showSizeChanger: false,
            showTotal: (total) => `共 ${total} 条`,
            onChange: (p) => setPage(p),
          }}
          expandable={{
            expandedRowRender: (record) => (
              <div style={{ padding: '12px 24px', background: '#fafafa' }}>
                <Text type="secondary">详细信息:</Text>
                <pre style={{ margin: '8px 0', fontSize: 12 }}>
                  {JSON.stringify(record.metadata, null, 2)}
                </pre>
              </div>
            ),
          }}
        />
      </Card>
    </div>
  )
}
