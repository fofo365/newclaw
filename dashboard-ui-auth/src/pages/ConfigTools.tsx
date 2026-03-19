import { useEffect, useState } from 'react'
import { Card, Table, Switch, Tag, Typography, message, Space, Button } from 'antd'
import { ToolOutlined, SaveOutlined } from '@ant-design/icons'
import { getToolsConfig, type ToolInfo } from '../services/api'

const { Title, Text } = Typography

export default function ConfigTools() {
  const [loading, setLoading] = useState(true)
  const [tools, setTools] = useState<ToolInfo[]>([])
  const [enabledTools, setEnabledTools] = useState<Set<string>>(new Set())

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await getToolsConfig()
      setTools(res.data.tools)
      setEnabledTools(new Set(res.data.tools.filter(t => t.enabled).map(t => t.name)))
    } catch (error) {
      message.error('加载配置失败')
    } finally {
      setLoading(false)
    }
  }

  const toggleTool = (name: string, enabled: boolean) => {
    const newSet = new Set(enabledTools)
    if (enabled) {
      newSet.add(name)
    } else {
      newSet.delete(name)
    }
    setEnabledTools(newSet)
  }

  const handleSave = async () => {
    try {
      // await updateToolsConfig({ enabled_tools: Array.from(enabledTools) })
      message.success('配置已保存（演示模式）')
    } catch (error) {
      message.error('保存失败')
    }
  }

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'file': return 'blue'
      case 'system': return 'orange'
      case 'web': return 'green'
      default: return 'default'
    }
  }

  const columns = [
    {
      title: '工具名称',
      dataIndex: 'display_name',
      key: 'display_name',
      render: (text: string, record: ToolInfo) => (
        <Space>
          <ToolOutlined />
          <span>{text}</span>
          <Tag color="default">{record.name}</Tag>
        </Space>
      ),
    },
    {
      title: '类别',
      dataIndex: 'category',
      key: 'category',
      render: (category: string) => <Tag color={getCategoryColor(category)}>{category}</Tag>,
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
    },
    {
      title: '参数数量',
      dataIndex: 'parameters',
      key: 'params',
      render: (params: ToolInfo['parameters']) => params.length,
    },
    {
      title: '启用',
      dataIndex: 'name',
      key: 'enabled',
      render: (name: string) => (
        <Switch
          checked={enabledTools.has(name)}
          onChange={(checked) => toggleTool(name, checked)}
        />
      ),
    },
  ]

  return (
    <div>
      <div className="page-header">
        <Title level={2}>工具配置</Title>
        <Text type="secondary">配置和启用/禁用工具</Text>
      </div>

      <Card
        title="可用工具"
        extra={
          <Button type="primary" icon={<SaveOutlined />} onClick={handleSave}>
            保存配置
          </Button>
        }
        loading={loading}
      >
        <Table
          dataSource={tools}
          columns={columns}
          rowKey="name"
          pagination={false}
          expandable={{
            expandedRowRender: (record) => (
              <div style={{ padding: '12px 24px' }}>
                <Text strong>参数详情:</Text>
                <ul style={{ marginTop: 8, marginBottom: 0 }}>
                  {record.parameters.map(param => (
                    <li key={param.name}>
                      <code>{param.name}</code>
                      {param.required && <Tag color="red" style={{ marginLeft: 8 }}>必填</Tag>}
                      <span style={{ color: '#666', marginLeft: 8 }}>
                        ({param.type}) {param.description}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            ),
          }}
        />
      </Card>

      <Card title="统计" style={{ marginTop: 16 }}>
        <Space size="large">
          <div>
            <Text type="secondary">总工具数</Text>
            <br />
            <Text strong style={{ fontSize: 24 }}>{tools.length}</Text>
          </div>
          <div>
            <Text type="secondary">已启用</Text>
            <br />
            <Text strong style={{ fontSize: 24, color: '#52c41a' }}>{enabledTools.size}</Text>
          </div>
          <div>
            <Text type="secondary">已禁用</Text>
            <br />
            <Text strong style={{ fontSize: 24, color: '#ff4d4f' }}>{tools.length - enabledTools.size}</Text>
          </div>
        </Space>
      </Card>
    </div>
  )
}
