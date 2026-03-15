import { useEffect, useState } from 'react'
import { Card, Table, Button, Space, Tag, Modal, Form, Input, Select, message, Popconfirm, Typography } from 'antd'
import { PlusOutlined, DeleteOutlined, EditOutlined } from '@ant-design/icons'
import { listUsers, createUser, deleteUser, type UserInfo } from '../services/api'
import dayjs from 'dayjs'

const { Title, Text } = Typography
const { Option } = Select

export default function AdminUsers() {
  const [loading, setLoading] = useState(true)
  const [users, setUsers] = useState<UserInfo[]>([])
  const [modalVisible, setModalVisible] = useState(false)
  const [form] = Form.useForm()
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    loadUsers()
  }, [])

  const loadUsers = async () => {
    setLoading(true)
    try {
      const res = await listUsers()
      setUsers(res.data.users)
    } catch (error) {
      message.error('加载用户失败')
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async (values: { username: string; password: string; email?: string; role?: string }) => {
    setSubmitting(true)
    try {
      const res = await createUser(values)
      setUsers([...users, res.data])
      setModalVisible(false)
      form.resetFields()
      message.success('用户创建成功')
    } catch (error) {
      message.error('创建失败')
    } finally {
      setSubmitting(false)
    }
  }

  const handleDelete = async (id: string) => {
    try {
      await deleteUser(id)
      setUsers(users.filter(u => u.id !== id))
      message.success('用户已删除')
    } catch (error) {
      message.error('删除失败')
    }
  }

  const getRoleColor = (role: string) => {
    switch (role) {
      case 'admin': return 'red'
      case 'user': return 'blue'
      case 'readonly': return 'default'
      default: return 'default'
    }
  }

  const columns = [
    {
      title: '用户名',
      dataIndex: 'username',
      key: 'username',
      render: (name: string, record: UserInfo) => (
        <Space>
          <Text strong>{name}</Text>
          {record.id === 'admin' && <Tag color="gold">系统</Tag>}
        </Space>
      ),
    },
    {
      title: '邮箱',
      dataIndex: 'email',
      key: 'email',
      render: (email?: string) => email || '-',
    },
    {
      title: '角色',
      dataIndex: 'role',
      key: 'role',
      render: (role: string) => <Tag color={getRoleColor(role)}>{role}</Tag>,
    },
    {
      title: '状态',
      dataIndex: 'is_active',
      key: 'is_active',
      render: (active: boolean) => (
        <Tag color={active ? 'success' : 'error'}>{active ? '活跃' : '禁用'}</Tag>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (ts: string) => dayjs(ts).format('YYYY-MM-DD HH:mm'),
    },
    {
      title: '最后登录',
      dataIndex: 'last_login',
      key: 'last_login',
      render: (ts?: string) => ts ? dayjs(ts).format('YYYY-MM-DD HH:mm') : '从未',
    },
    {
      title: '配额使用',
      key: 'quota',
      render: (_: unknown, record: UserInfo) => (
        <Space direction="vertical" size="small">
          <Text type="secondary" style={{ fontSize: 12 }}>
            请求: {record.quota.used_requests} / {record.quota.max_requests_per_day || '∞'}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Token: {record.quota.used_tokens} / {record.quota.max_tokens_per_day || '∞'}
          </Text>
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: UserInfo) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} disabled>
            编辑
          </Button>
          <Popconfirm
            title="确定要删除此用户吗？"
            onConfirm={() => handleDelete(record.id)}
            okText="确定"
            cancelText="取消"
          >
            <Button size="small" danger icon={<DeleteOutlined />} disabled={record.id === 'admin'}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ]

  return (
    <div>
      <div className="page-header">
        <Title level={2}>用户管理</Title>
        <Text type="secondary">管理系统用户和权限</Text>
      </div>

      <Card
        title={`用户列表 (${users.length})`}
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setModalVisible(true)}>
            添加用户
          </Button>
        }
      >
        <Table
          dataSource={users}
          columns={columns}
          rowKey="id"
          loading={loading}
          pagination={{ pageSize: 10 }}
        />
      </Card>

      {/* 创建用户弹窗 */}
      <Modal
        title="添加用户"
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
        >
          <Form.Item
            name="username"
            label="用户名"
            rules={[
              { required: true, message: '请输入用户名' },
              { min: 3, message: '用户名至少 3 个字符' },
            ]}
          >
            <Input placeholder="请输入用户名" />
          </Form.Item>
          <Form.Item
            name="email"
            label="邮箱"
            rules={[{ type: 'email', message: '请输入有效的邮箱地址' }]}
          >
            <Input placeholder="请输入邮箱（可选）" />
          </Form.Item>
          <Form.Item
            name="password"
            label="密码"
            rules={[
              { required: true, message: '请输入密码' },
              { min: 8, message: '密码至少 8 个字符' },
            ]}
          >
            <Input.Password placeholder="请输入密码" />
          </Form.Item>
          <Form.Item name="role" label="角色" initialValue="user">
            <Select>
              <Option value="user">用户 (user)</Option>
              <Option value="admin">管理员 (admin)</Option>
              <Option value="readonly">只读 (readonly)</Option>
            </Select>
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
    </div>
  )
}
