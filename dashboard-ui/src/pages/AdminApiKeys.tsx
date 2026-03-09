import { useEffect, useState } from 'react'
import { Card, Table, Button, Space, Tag, Modal, Form, Input, Select, message, Typography, Checkbox } from 'antd'
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons'
import { listApiKeys, createApiKey, revokeApiKey, type ApiKeyInfo } from '../services/api'
import dayjs from 'dayjs'

const { Title, Text, Paragraph } = Typography
const { Option } = Select

export default function AdminApiKeys() {
  const [loading, setLoading] = useState(true)
  const [apiKeys, setApiKeys] = useState<ApiKeyInfo[]>([])
  const [modalVisible, setModalVisible] = useState(false)
  const [form] = Form.useForm()
  const [submitting, setSubmitting] = useState(false)
  const [newKey, setNewKey] = useState<string | null>(null)

  useEffect(() => {
    loadApiKeys()
  }, [])

  const loadApiKeys = async () => {
    setLoading(true)
    try {
      const res = await listApiKeys()
      setApiKeys(res.data)
    } catch (error) {
      message.error('加载 API Keys 失败')
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async (values: { name: string; expires_in_days?: number; permissions?: string[] }) => {
    setSubmitting(true)
    try {
      const res = await createApiKey(values)
      setApiKeys([...apiKeys, res.data.info])
      setNewKey(res.data.key)
      setModalVisible(false)
      form.resetFields()
      message.success('API Key 创建成功')
    } catch (error) {
      message.error('创建失败')
    } finally {
      setSubmitting(false)
    }
  }

  const handleRevoke = async (id: string) => {
    try {
      await revokeApiKey(id)
      setApiKeys(apiKeys.map(k => k.id === id ? { ...k, is_active: false } : k))
      message.success('API Key 已撤销')
    } catch (error) {
      message.error('撤销失败')
    }
  }

  const copyKey = () => {
    if (newKey) {
      navigator.clipboard.writeText(newKey)
      message.success('已复制到剪贴板')
    }
  }

  const columns = [
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: '前缀',
      dataIndex: 'prefix',
      key: 'prefix',
      render: (prefix: string) => <Text code>{prefix}...</Text>,
    },
    {
      title: '状态',
      dataIndex: 'is_active',
      key: 'is_active',
      render: (active: boolean) => (
        <Tag color={active ? 'success' : 'error'}>{active ? '活跃' : '已撤销'}</Tag>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (ts: string) => dayjs(ts).format('YYYY-MM-DD HH:mm'),
    },
    {
      title: '过期时间',
      dataIndex: 'expires_at',
      key: 'expires_at',
      render: (ts?: string) => ts ? dayjs(ts).format('YYYY-MM-DD HH:mm') : '永不过期',
    },
    {
      title: '最后使用',
      dataIndex: 'last_used',
      key: 'last_used',
      render: (ts?: string) => ts ? dayjs(ts).format('YYYY-MM-DD HH:mm') : '从未使用',
    },
    {
      title: '权限',
      dataIndex: 'permissions',
      key: 'permissions',
      render: (perms: string[]) => (
        <Space size="small" wrap>
          {perms.map(p => <Tag key={p}>{p}</Tag>)}
        </Space>
      ),
    },
    {
      title: '使用统计',
      key: 'usage',
      render: (_: unknown, record: ApiKeyInfo) => (
        <Space direction="vertical" size="small">
          <Text type="secondary" style={{ fontSize: 12 }}>
            总请求: {record.usage.total_requests}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            24h: {record.usage.last_24h_requests} 请求 / {record.usage.last_24h_tokens} tokens
          </Text>
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: ApiKeyInfo) => (
        <Button
          size="small"
          danger
          icon={<DeleteOutlined />}
          onClick={() => handleRevoke(record.id)}
          disabled={!record.is_active}
        >
          撤销
        </Button>
      ),
    },
  ]

  return (
    <div>
      <div className="page-header">
        <Title level={2}>API Key 管理</Title>
        <Text type="secondary">管理 API 访问密钥</Text>
      </div>

      <Card
        title={`API Keys (${apiKeys.length})`}
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setModalVisible(true)}>
            创建 API Key
          </Button>
        }
      >
        <Table
          dataSource={apiKeys}
          columns={columns}
          rowKey="id"
          loading={loading}
          pagination={{ pageSize: 10 }}
        />
      </Card>

      {/* 创建 API Key 弹窗 */}
      <Modal
        title="创建 API Key"
        open={modalVisible}
        onCancel={() => {
          setModalVisible(false)
          form.resetFields()
        }}
        footer={null}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleCreate}
          initialValues={{ permissions: ['chat', 'tools'] }}
        >
          <Form.Item
            name="name"
            label="名称"
            rules={[{ required: true, message: '请输入名称' }]}
          >
            <Input placeholder="例如：生产环境 Key" />
          </Form.Item>
          <Form.Item name="expires_in_days" label="有效期">
            <Select allowClear placeholder="永不过期">
              <Option value={7}>7 天</Option>
              <Option value={30}>30 天</Option>
              <Option value={90}>90 天</Option>
              <Option value={365}>1 年</Option>
            </Select>
          </Form.Item>
          <Form.Item name="permissions" label="权限">
            <Checkbox.Group>
              <Space direction="vertical">
                <Checkbox value="chat">对话 (chat)</Checkbox>
                <Checkbox value="tools">工具 (tools)</Checkbox>
                <Checkbox value="admin">管理 (admin)</Checkbox>
              </Space>
            </Checkbox.Group>
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                创建
              </Button>
              <Button onClick={() => setModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 显示新 Key 的弹窗 */}
      <Modal
        title="API Key 创建成功"
        open={!!newKey}
        onCancel={() => setNewKey(null)}
        footer={null}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Text type="danger">⚠️ 请立即复制此 Key，关闭后将无法再次查看！</Text>
          <Paragraph
            copyable={{
              text: newKey || '',
              onCopy: copyKey,
            }}
            style={{
              background: '#f5f5f5',
              padding: 12,
              borderRadius: 4,
              wordBreak: 'break-all',
            }}
          >
            {newKey}
          </Paragraph>
          <Button type="primary" block onClick={() => setNewKey(null)}>
            我已保存，关闭
          </Button>
        </Space>
      </Modal>
    </div>
  )
}
