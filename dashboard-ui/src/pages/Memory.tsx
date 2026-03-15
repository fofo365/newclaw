import { useEffect, useState } from 'react'
import { Card, Table, Button, Tag, Space, Modal, Form, Input, Select, message, Typography, Tabs, List, Avatar } from 'antd'
import {
  PlusOutlined,
  SearchOutlined,
  DeleteOutlined,
  SyncOutlined,
  CloudServerOutlined,
  DatabaseOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'

const { Title, Text, Paragraph } = Typography
const { TextArea } = Input

interface Memory {
  id: string
  content: string
  importance: number
  created_at: string
  updated_at: string
  tags: string[]
}

interface FederationNode {
  id: string
  name: string
  status: string
  last_sync?: string
  latency_ms?: number
}

export default function Memory() {
  const [memories, setMemories] = useState<Memory[]>([])
  const [federationNodes, setFederationNodes] = useState<FederationNode[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'memories' | 'federation'>('memories')
  const [storeModalVisible, setStoreModalVisible] = useState(false)
  const [searchModalVisible, setSearchModalVisible] = useState(false)
  const [storeForm] = Form.useForm()
  const [searchForm] = Form.useForm()
  const [searchResults, setSearchResults] = useState<Memory[]>([])

  const fetchData = async () => {
    setLoading(true)
    try {
      const res = await fetch('/api/memories?limit=50')
      if (res.ok) {
        const data = await res.json()
        setMemories(data.memories || [])
      }
      
      const fedRes = await fetch('/api/federation/status')
      if (fedRes.ok) {
        const data = await fedRes.json()
        setFederationNodes(data.nodes || [])
      }
    } catch (error) {
      console.error('Failed to fetch data:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
  }, [])

  const handleStoreMemory = async (values: any) => {
    try {
      const res = await fetch('/api/memories', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          content: values.content,
          importance: values.importance,
          tags: values.tags?.split(',').map((t: string) => t.trim()).filter(Boolean),
        }),
      })
      if (res.ok) {
        message.success('记忆存储成功')
        setStoreModalVisible(false)
        storeForm.resetFields()
        fetchData()
      } else {
        message.error('存储失败')
      }
    } catch (error) {
      message.error('存储失败')
    }
  }

  const handleSearchMemory = async (values: any) => {
    try {
      const res = await fetch('/api/memories/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(values),
      })
      if (res.ok) {
        const data = await res.json()
        setSearchResults(data.results || [])
        message.success(`找到 ${data.total || 0} 条记忆`)
      }
    } catch (error) {
      message.error('搜索失败')
    }
  }

  const handleDeleteMemory = async (id: string) => {
    try {
      const res = await fetch(`/api/memories/${id}`, { method: 'DELETE' })
      if (res.ok) {
        message.success('记忆已删除')
        fetchData()
      }
    } catch (error) {
      message.error('删除失败')
    }
  }

  const handleSync = async () => {
    try {
      const res = await fetch('/api/federation/sync', { method: 'POST' })
      if (res.ok) {
        const data = await res.json()
        message.success(data.message || '同步完成')
        fetchData()
      }
    } catch (error) {
      message.error('同步失败')
    }
  }

  const columns: ColumnsType<Memory> = [
    {
      title: '内容',
      dataIndex: 'content',
      key: 'content',
      ellipsis: true,
      render: (text: string) => (
        <Paragraph ellipsis={{ rows: 2 }} style={{ margin: 0 }}>
          {text}
        </Paragraph>
      ),
    },
    {
      title: '重要性',
      dataIndex: 'importance',
      key: 'importance',
      width: 100,
      render: (value: number) => (
        <Tag color={value > 0.7 ? 'red' : value > 0.4 ? 'orange' : 'default'}>
          {value?.toFixed(2) || '0.50'}
        </Tag>
      ),
    },
    {
      title: '标签',
      dataIndex: 'tags',
      key: 'tags',
      width: 150,
      render: (tags: string[]) => (
        <Space size={4} wrap>
          {(tags || []).slice(0, 3).map(tag => (
            <Tag key={tag} color="blue">{tag}</Tag>
          ))}
          {(tags || []).length > 3 && <Tag>+{(tags || []).length - 3}</Tag>}
        </Space>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 180,
      render: (t: string) => new Date(t).toLocaleString(),
    },
    {
      title: '操作',
      key: 'action',
      width: 80,
      render: (_, record) => (
        <Button size="small" danger icon={<DeleteOutlined />} onClick={() => handleDeleteMemory(record.id)} />
      ),
    },
  ]

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title level={2}>记忆管理</Title>
          <Text type="secondary">记忆存储、搜索与联邦同步</Text>
        </div>
        <Space>
          <Button icon={<SearchOutlined />} onClick={() => setSearchModalVisible(true)}>
            搜索
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setStoreModalVisible(true)}>
            存储记忆
          </Button>
        </Space>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Space size="large">
          <Button type={activeTab === 'memories' ? 'primary' : 'default'} onClick={() => setActiveTab('memories')}>
            <DatabaseOutlined /> 记忆列表
          </Button>
          <Button type={activeTab === 'federation' ? 'primary' : 'default'} onClick={() => setActiveTab('federation')}>
            <CloudServerOutlined /> 联邦状态
          </Button>
        </Space>
      </Card>

      {activeTab === 'memories' && (
        <Card loading={loading}>
          <Table columns={columns} dataSource={memories} rowKey="id" pagination={{ pageSize: 10 }} />
        </Card>
      )}

      {activeTab === 'federation' && (
        <Card loading={loading} extra={<Button icon={<SyncOutlined />} onClick={handleSync}>同步</Button>}>
          <List
            itemLayout="horizontal"
            dataSource={federationNodes}
            renderItem={item => (
              <List.Item>
                <List.Item.Meta
                  avatar={<Avatar icon={<CloudServerOutlined />} />}
                  title={item.name}
                  description={
                    <Space>
                      <Tag color={item.status === 'online' ? 'success' : 'default'}>
                        {item.status}
                      </Tag>
                      {item.latency_ms && <Text type="secondary">{item.latency_ms}ms</Text>}
                      {item.last_sync && <Text type="secondary">最后同步: {new Date(item.last_sync).toLocaleString()}</Text>}
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
          {federationNodes.length === 0 && (
            <div style={{ textAlign: 'center', padding: '40px 0' }}>
              <Text type="secondary">暂无联邦节点</Text>
            </div>
          )}
        </Card>
      )}

      <Modal
        title="存储记忆"
        open={storeModalVisible}
        onOk={() => storeForm.submit()}
        onCancel={() => setStoreModalVisible(false)}
      >
        <Form form={storeForm} layout="vertical" onFinish={handleStoreMemory}>
          <Form.Item name="content" label="记忆内容" rules={[{ required: true }]}>
            <TextArea rows={4} placeholder="输入要存储的记忆内容" />
          </Form.Item>
          <Form.Item name="importance" label="重要性 (0-1)">
            <Input type="number" min={0} max={1} step={0.1} placeholder="0.5" />
          </Form.Item>
          <Form.Item name="tags" label="标签 (逗号分隔)">
            <Input placeholder="tag1, tag2, tag3" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="搜索记忆"
        open={searchModalVisible}
        onOk={() => searchForm.submit()}
        onCancel={() => {
          setSearchModalVisible(false)
          setSearchResults([])
        }}
        width={700}
      >
        <Form form={searchForm} layout="vertical" onFinish={handleSearchMemory}>
          <Form.Item name="query" label="搜索关键词" rules={[{ required: true }]}>
            <Input placeholder="输入搜索关键词" />
          </Form.Item>
          <Form.Item name="limit" label="结果数量">
            <Input type="number" min={1} max={100} placeholder="10" />
          </Form.Item>
        </Form>
        
        {searchResults.length > 0 && (
          <div style={{ marginTop: 16 }}>
            <Title level={5}>搜索结果</Title>
            <List
              size="small"
              dataSource={searchResults}
              renderItem={item => (
                <List.Item>
                  <Text>{item.content}</Text>
                </List.Item>
              )}
            />
          </div>
        )}
      </Modal>
    </div>
  )
}