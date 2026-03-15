import { useEffect, useState } from 'react'
import { Card, Table, Button, Tag, Space, Modal, Form, Input, Select, message, Typography, Progress } from 'antd'
import {
  PlusOutlined,
  StopOutlined,
  ReloadOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  PlayCircleOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'

const { Title, Text } = Typography

interface Task {
  id: string
  name: string
  task_type: string
  status: string
  progress: number
  created_at: string
  updated_at: string
  result?: string
  error?: string
}

interface DagWorkflow {
  id: string
  name: string
  status: string
  nodes: { id: string; name: string; status: string }[]
  created_at: string
}

interface Schedule {
  id: string
  name: string
  cron_expression: string
  task_type: string
  enabled: boolean
  last_run?: string
  next_run?: string
}

const taskStatusColor: Record<string, string> = {
  pending: 'default',
  running: 'processing',
  completed: 'success',
  failed: 'error',
  cancelled: 'warning',
}

export default function Tasks() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [dags, setDags] = useState<DagWorkflow[]>([])
  const [schedules, setSchedules] = useState<Schedule[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'tasks' | 'dags' | 'schedules'>('tasks')
  const [createModalVisible, setCreateModalVisible] = useState(false)
  const [createForm] = Form.useForm()

  const fetchData = async () => {
    setLoading(true)
    try {
      // Fetch tasks
      const tasksRes = await fetch('/api/tasks')
      if (tasksRes.ok) {
        const data = await tasksRes.json()
        setTasks(data.tasks || [])
      }
      
      // Fetch DAGs
      const dagsRes = await fetch('/api/dags')
      if (dagsRes.ok) {
        const data = await dagsRes.json()
        setDags(data || [])
      }
      
      // Fetch schedules
      const schedulesRes = await fetch('/api/schedules')
      if (schedulesRes.ok) {
        const data = await schedulesRes.json()
        setSchedules(data.schedules || [])
      }
    } catch (error) {
      console.error('Failed to fetch data:', error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchData, 30000)
    return () => clearInterval(interval)
  }, [])

  const handleCreateTask = async (values: any) => {
    try {
      const res = await fetch('/api/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(values),
      })
      if (res.ok) {
        message.success('任务创建成功')
        setCreateModalVisible(false)
        createForm.resetFields()
        fetchData()
      } else {
        message.error('创建失败')
      }
    } catch (error) {
      message.error('创建失败')
    }
  }

  const handleCancelTask = async (id: string) => {
    try {
      const res = await fetch(`/api/tasks/${id}/cancel`, { method: 'POST' })
      if (res.ok) {
        message.success('任务已取消')
        fetchData()
      }
    } catch (error) {
      message.error('操作失败')
    }
  }

  const handleRunDag = async (id: string) => {
    try {
      const res = await fetch(`/api/dags/${id}/run`, { method: 'POST' })
      if (res.ok) {
        message.success('DAG 工作流已启动')
        fetchData()
      }
    } catch (error) {
      message.error('启动失败')
    }
  }

  const handleDeleteSchedule = async (id: string) => {
    try {
      const res = await fetch(`/api/schedules/${id}`, { method: 'DELETE' })
      if (res.ok) {
        message.success('调度任务已删除')
        fetchData()
      }
    } catch (error) {
      message.error('删除失败')
    }
  }

  const taskColumns: ColumnsType<Task> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 100, ellipsis: true },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'task_type', key: 'task_type', width: 100 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: string) => (
        <Tag color={taskStatusColor[status] || 'default'}>
          {status.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: '进度',
      dataIndex: 'progress',
      key: 'progress',
      width: 120,
      render: (progress: number) => <Progress percent={Math.round(progress * 100)} size="small" />,
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
      width: 100,
      render: (_, record) => (
        record.status === 'running' || record.status === 'pending' ? (
          <Button size="small" danger onClick={() => handleCancelTask(record.id)}>
            取消
          </Button>
        ) : null
      ),
    },
  ]

  const dagColumns: ColumnsType<DagWorkflow> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 100, ellipsis: true },
    { title: '名称', dataIndex: 'name', key: 'name' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: string) => (
        <Tag color={taskStatusColor[status] || 'default'}>
          {status.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: '节点数',
      key: 'nodes',
      width: 80,
      render: (_, record) => record.nodes?.length || 0,
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
      width: 100,
      render: (_, record) => (
        <Button size="small" type="primary" onClick={() => handleRunDag(record.id)}>
          执行
        </Button>
      ),
    },
  ]

  const scheduleColumns: ColumnsType<Schedule> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 100, ellipsis: true },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: 'Cron 表达式', dataIndex: 'cron_expression', key: 'cron', width: 120 },
    { title: '类型', dataIndex: 'task_type', key: 'task_type', width: 100 },
    {
      title: '状态',
      dataIndex: 'enabled',
      key: 'enabled',
      width: 80,
      render: (enabled: boolean) => (
        <Tag color={enabled ? 'success' : 'default'}>
          {enabled ? '启用' : '禁用'}
        </Tag>
      ),
    },
    {
      title: '下次执行',
      dataIndex: 'next_run',
      key: 'next_run',
      width: 180,
      render: (t?: string) => t ? new Date(t).toLocaleString() : '-',
    },
    {
      title: '操作',
      key: 'action',
      width: 80,
      render: (_, record) => (
        <Button size="small" danger onClick={() => handleDeleteSchedule(record.id)}>
          删除
        </Button>
      ),
    },
  ]

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title level={2}>任务管理</Title>
          <Text type="secondary">任务、DAG工作流、调度管理</Text>
        </div>
        <Space>
          <Button icon={<ReloadOutlined />} onClick={fetchData}>刷新</Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalVisible(true)}>
            创建任务
          </Button>
        </Space>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Space size="large">
          <Button type={activeTab === 'tasks' ? 'primary' : 'default'} onClick={() => setActiveTab('tasks')}>
            任务列表
          </Button>
          <Button type={activeTab === 'dags' ? 'primary' : 'default'} onClick={() => setActiveTab('dags')}>
            DAG 工作流
          </Button>
          <Button type={activeTab === 'schedules' ? 'primary' : 'default'} onClick={() => setActiveTab('schedules')}>
            调度任务
          </Button>
        </Space>
      </Card>

      <Card loading={loading}>
        {activeTab === 'tasks' && (
          <Table columns={taskColumns} dataSource={tasks} rowKey="id" pagination={{ pageSize: 10 }} />
        )}
        {activeTab === 'dags' && (
          <Table columns={dagColumns} dataSource={dags} rowKey="id" pagination={{ pageSize: 10 }} />
        )}
        {activeTab === 'schedules' && (
          <Table columns={scheduleColumns} dataSource={schedules} rowKey="id" pagination={{ pageSize: 10 }} />
        )}
      </Card>

      <Modal
        title="创建任务"
        open={createModalVisible}
        onOk={() => createForm.submit()}
        onCancel={() => setCreateModalVisible(false)}
      >
        <Form form={createForm} layout="vertical" onFinish={handleCreateTask}>
          <Form.Item name="name" label="任务名称" rules={[{ required: true }]}>
            <Input placeholder="输入任务名称" />
          </Form.Item>
          <Form.Item name="task_type" label="任务类型" rules={[{ required: true }]}>
            <Select placeholder="选择任务类型">
              <Select.Option value="chat">对话</Select.Option>
              <Select.Option value="tool_call">工具调用</Select.Option>
              <Select.Option value="workflow">工作流</Select.Option>
              <Select.Option value="scheduled">定时任务</Select.Option>
            </Select>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  )
}