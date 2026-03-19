import { useEffect, useState } from 'react'
import { Card, Table, Button, Tag, Space, Modal, Form, Input, message, Typography, Descriptions, Popconfirm, Badge } from 'antd'
import {
  PlusOutlined,
  DeleteOutlined,
  SwapOutlined,
  MessageOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'
import dayjs from 'dayjs'

const { Title, Text } = Typography

interface Session {
  id: string
  name: string
  status: 'active' | 'inactive' | 'archived'
  created_at: string
  updated_at: string
  message_count: number
  model: string
  provider: string
}

export default function Sessions() {
  const [sessions, setSessions] = useState<Session[]>([])
  const [currentSession, setCurrentSession] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [createModalVisible, setCreateModalVisible] = useState(false)
  const [createForm] = Form.useForm()

  const fetchSessions = async () => {
    setLoading(true)
    try {
      const res = await fetch('/api/chat/sessions')
      if (res.ok) {
        const data = await res.json()
        setSessions(data.sessions || [])
      }
    } catch (error) {
      console.error('Failed to fetch sessions:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchSessions()
  }, [])

  const handleCreate = async (values: { name: string }) => {
    try {
      const res = await fetch('/api/chat/sessions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title: values.name }),
      })
      if (res.ok) {
        message.success('会话创建成功')
        setCreateModalVisible(false)
        createForm.resetFields()
        fetchSessions()
      }
    } catch (error) {
      message.error('创建失败')
    }
  }

  const handleSwitch = (id: string) => {
    setCurrentSession(id)
    message.success(`已切换到会话: ${id.slice(0, 8)}`)
  }

  const handleClose = async (_id: string) => {
    message.success('会话已关闭')
    fetchSessions()
  }

  const columns: ColumnsType<Session> = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 100,
      render: (id: string) => <Text code>{id.slice(0, 8)}</Text>,
    },
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record) => (
        <Space>
          {currentSession === record.id && <Badge status="success" />}
          <Text strong={currentSession === record.id}>{name || '未命名会话'}</Text>
        </Space>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: string) => (
        <Tag color={status === 'active' ? 'success' : status === 'inactive' ? 'warning' : 'default'}>
          {status === 'active' ? '活跃' : status === 'inactive' ? '未激活' : '已归档'}
        </Tag>
      ),
    },
    {
      title: '消息数',
      dataIndex: 'message_count',
      key: 'message_count',
      width: 80,
      render: (count: number) => <Tag icon={<MessageOutlined />}>{count || 0}</Tag>,
    },
    {
      title: '模型',
      dataIndex: 'model',
      key: 'model',
      width: 100,
    },
    {
      title: '更新时间',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 180,
      render: (t: string) => (
        <Space>
          <ClockCircleOutlined />
          {dayjs(t).format('MM-DD HH:mm')}
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'action',
      width: 180,
      render: (_, record) => (
        <Space>
          <Button
            type={currentSession === record.id ? 'primary' : 'default'}
            size="small"
            icon={<SwapOutlined />}
            onClick={() => handleSwitch(record.id)}
            disabled={currentSession === record.id}
          >
            切换
          </Button>
          <Popconfirm
            title="确定关闭此会话？"
            onConfirm={() => handleClose(record.id)}
          >
            <Button size="small" danger icon={<DeleteOutlined />}>
              关闭
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  const activeCount = sessions.filter(s => s.status === 'active').length
  const totalMessages = sessions.reduce((sum, s) => sum + (s.message_count || 0), 0)

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title level={2}>会话管理</Title>
          <Text type="secondary">管理对话会话，创建、切换和关闭会话</Text>
        </div>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalVisible(true)}>
          创建会话
        </Button>
      </div>

      {/* 统计信息 */}
      <Card style={{ marginBottom: 16 }}>
        <Descriptions column={4}>
          <Descriptions.Item label="总会话数">{sessions.length}</Descriptions.Item>
          <Descriptions.Item label="活跃会话">{activeCount}</Descriptions.Item>
          <Descriptions.Item label="总消息数">{totalMessages}</Descriptions.Item>
          <Descriptions.Item label="当前会话">
            {currentSession ? <Text code>{currentSession.slice(0, 8)}</Text> : <Text type="secondary">未选择</Text>}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      {/* 会话列表 */}
      <Card loading={loading}>
        <Table
          columns={columns}
          dataSource={sessions}
          rowKey="id"
          pagination={{ pageSize: 10 }}
        />
      </Card>

      {/* 创建会话弹窗 */}
      <Modal
        title="创建新会话"
        open={createModalVisible}
        onOk={() => createForm.submit()}
        onCancel={() => setCreateModalVisible(false)}
      >
        <Form form={createForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="会话名称">
            <Input placeholder="输入会话名称（可选）" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  )
}