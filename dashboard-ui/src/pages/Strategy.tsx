import { useState } from 'react'
import { Card, Table, Button, Tag, Space, Modal, Form, InputNumber, message, Typography, Descriptions, Divider } from 'antd'
import {
  SettingOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons'
import type { ColumnsType } from 'antd/es/table'

const { Title, Text } = Typography

interface Strategy {
  id: string
  name: string
  description: string
  enabled: boolean
  priority: number
  config: Record<string, any>
}

interface StrategyConfig {
  max_tokens: number
  time_window_secs: number
  threshold: number
  decay_factor: number
  cluster_size: number
}

const strategies: Strategy[] = [
  {
    id: 'smart',
    name: 'Smart',
    description: '智能上下文选择，根据重要性动态选择内容',
    enabled: true,
    priority: 1,
    config: { max_tokens: 4096, threshold: 0.7 },
  },
  {
    id: 'time_decay',
    name: 'Time Decay',
    description: '基于时间的衰减策略，新内容权重更高',
    enabled: false,
    priority: 2,
    config: { max_tokens: 4096, time_window_secs: 3600, decay_factor: 0.95 },
  },
  {
    id: 'semantic_cluster',
    name: 'Semantic Cluster',
    description: '语义聚类策略，按主题分组选择内容',
    enabled: false,
    priority: 3,
    config: { max_tokens: 4096, cluster_size: 5, threshold: 0.8 },
  },
]

export default function Strategy() {
  const [currentStrategy, setCurrentStrategy] = useState<string>('smart')
  const [config, setConfig] = useState<StrategyConfig>({
    max_tokens: 4096,
    time_window_secs: 3600,
    threshold: 0.7,
    decay_factor: 0.95,
    cluster_size: 5,
  })
  const [configModalVisible, setConfigModalVisible] = useState(false)
  const [configForm] = Form.useForm()

  const handleSelectStrategy = (id: string) => {
    setCurrentStrategy(id)
    message.success(`已切换到策略: ${id}`)
  }

  const handleConfigSave = (values: any) => {
    setConfig({ ...config, ...values })
    setConfigModalVisible(false)
    message.success('配置已保存')
  }

  const columns: ColumnsType<Strategy> = [
    {
      title: '策略名称',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record) => (
        <Space>
          {currentStrategy === record.id && <CheckCircleOutlined style={{ color: '#52c41a' }} />}
          <Text strong={currentStrategy === record.id}>{name}</Text>
        </Space>
      ),
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
    },
    {
      title: '状态',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (_: boolean, record) => (
        <Tag color={currentStrategy === record.id ? 'success' : 'default'}>
          {currentStrategy === record.id ? '当前使用' : '可用'}
        </Tag>
      ),
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button
            type={currentStrategy === record.id ? 'primary' : 'default'}
            size="small"
            onClick={() => handleSelectStrategy(record.id)}
            disabled={currentStrategy === record.id}
          >
            选择
          </Button>
        </Space>
      ),
    },
  ]

  return (
    <div>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title level={2}>上下文策略</Title>
          <Text type="secondary">管理对话上下文的选择和压缩策略</Text>
        </div>
        <Button icon={<SettingOutlined />} onClick={() => setConfigModalVisible(true)}>
          全局配置
        </Button>
      </div>

      {/* 当前策略信息 */}
      <Card style={{ marginBottom: 16 }}>
        <Descriptions title="当前策略" column={3}>
          <Descriptions.Item label="策略名称">{currentStrategy.toUpperCase()}</Descriptions.Item>
          <Descriptions.Item label="最大 Token">{config.max_tokens}</Descriptions.Item>
          <Descriptions.Item label="阈值">{config.threshold}</Descriptions.Item>
        </Descriptions>
      </Card>

      {/* 策略列表 */}
      <Card title="可用策略">
        <Table
          columns={columns}
          dataSource={strategies}
          rowKey="id"
          pagination={false}
        />
      </Card>

      {/* 策略详情 */}
      <Divider />
      <Card title="策略详情">
        <div style={{ marginBottom: 16 }}>
          <Title level={5}>Smart 策略</Title>
          <Text type="secondary">
            智能上下文选择策略，根据内容重要性和相关性动态选择最相关的上下文内容。
            适用于大多数场景，平衡了准确性和效率。
          </Text>
        </div>
        <div style={{ marginBottom: 16 }}>
          <Title level={5}>Time Decay 策略</Title>
          <Text type="secondary">
            基于时间的衰减策略，较新的内容会获得更高的权重。
            适用于需要关注最新信息的场景，如实时监控、新闻摘要等。
          </Text>
        </div>
        <div>
          <Title level={5}>Semantic Cluster 策略</Title>
          <Text type="secondary">
            语义聚类策略，将内容按语义相似性分组，从每个组中选择代表性内容。
            适用于需要全面覆盖多个主题的场景。
          </Text>
        </div>
      </Card>

      {/* 配置弹窗 */}
      <Modal
        title="全局配置"
        open={configModalVisible}
        onOk={() => configForm.submit()}
        onCancel={() => setConfigModalVisible(false)}
      >
        <Form
          form={configForm}
          layout="vertical"
          initialValues={config}
          onFinish={handleConfigSave}
        >
          <Form.Item name="max_tokens" label="最大 Token 数">
            <InputNumber min={1024} max={32000} step={1024} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="threshold" label="相关性阈值 (0-1)">
            <InputNumber min={0} max={1} step={0.1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="time_window_secs" label="时间窗口 (秒)">
            <InputNumber min={60} max={86400} step={60} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="decay_factor" label="衰减因子 (0-1)">
            <InputNumber min={0} max={1} step={0.05} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="cluster_size" label="聚类大小">
            <InputNumber min={1} max={20} step={1} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  )
}