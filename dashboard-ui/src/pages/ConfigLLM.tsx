import { useEffect, useState } from 'react'
import { Card, Form, Select, InputNumber, Input, Button, Space, message, Tag, Typography, Divider, Alert } from 'antd'
import { SaveOutlined, ReloadOutlined } from '@ant-design/icons'
import { getLLMConfig, updateLLMConfig, type LLMConfig, type ProviderInfo } from '../services/api'

const { Title, Text } = Typography
const { Option } = Select

export default function ConfigLLM() {
  const [form] = Form.useForm()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [providers, setProviders] = useState<ProviderInfo[]>([])
  const [selectedProvider, setSelectedProvider] = useState<string>('')

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await getLLMConfig()
      const config = res.data
      setProviders(config.providers)
      setSelectedProvider(config.provider)
      form.setFieldsValue({
        provider: config.provider,
        model: config.model,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
      })
    } catch (error) {
      message.error('加载配置失败')
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async (values: Partial<LLMConfig>) => {
    setSaving(true)
    try {
      await updateLLMConfig(values)
      message.success('配置已保存')
    } catch (error) {
      message.error('保存失败')
    } finally {
      setSaving(false)
    }
  }

  const currentProvider = providers.find(p => p.name === selectedProvider)

  return (
    <div>
      <div className="page-header">
        <Title level={2}>LLM 配置</Title>
        <Text type="secondary">配置大语言模型提供商和参数</Text>
      </div>

      {/* Provider 选择 */}
      <Card title="选择 Provider" style={{ marginBottom: 16 }} loading={loading}>
        <Alert
          message="API Key 配置"
          description="API Key 应通过环境变量配置，不建议在此界面保存。设置 OPENAI_API_KEY、ANTHROPIC_API_KEY 或 GLM_API_KEY 环境变量。"
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
        />
        
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: 12 }}>
          {providers.map(provider => (
            <Card
              key={provider.name}
              hoverable
              onClick={() => {
                setSelectedProvider(provider.name)
                form.setFieldsValue({ provider: provider.name })
              }}
              style={{
                borderColor: selectedProvider === provider.name ? '#1890ff' : undefined,
                borderWidth: selectedProvider === provider.name ? 2 : 1,
              }}
            >
              <div style={{ textAlign: 'center' }}>
                <Text strong>{provider.display_name}</Text>
                <br />
                {provider.configured ? (
                  <Tag color="success">已配置</Tag>
                ) : (
                  <Tag color="warning">未配置</Tag>
                )}
              </div>
            </Card>
          ))}
        </div>
      </Card>

      {/* 模型参数 */}
      <Card title="模型参数" loading={loading}>
        <Form
          form={form}
          layout="vertical"
          className="config-form"
          onFinish={handleSave}
        >
          <Form.Item name="provider" label="Provider" hidden>
            <Input />
          </Form.Item>

          <Form.Item name="model" label="模型" rules={[{ required: true }]}>
            <Select placeholder="选择模型">
              {currentProvider?.models.map(model => (
                <Option key={model} value={model}>{model}</Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item name="temperature" label="Temperature" tooltip="控制输出的随机性，0-1 之间">
            <InputNumber min={0} max={1} step={0.1} style={{ width: '100%' }} />
          </Form.Item>

          <Form.Item name="max_tokens" label="Max Tokens" tooltip="最大输出 token 数">
            <InputNumber min={1} max={128000} step={256} style={{ width: '100%' }} />
          </Form.Item>

          <Divider />

          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" icon={<SaveOutlined />} loading={saving}>
                保存配置
              </Button>
              <Button icon={<ReloadOutlined />} onClick={loadConfig}>
                重新加载
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      {/* Provider 详情 */}
      {currentProvider && (
        <Card title="Provider 详情" style={{ marginTop: 16 }}>
          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <Text type="secondary">名称: </Text>
              <Text code>{currentProvider.name}</Text>
            </div>
            <div>
              <Text type="secondary">状态: </Text>
              {currentProvider.configured ? (
                <Tag color="success">已配置 API Key</Tag>
              ) : (
                <Tag color="warning">未配置 API Key</Tag>
              )}
            </div>
            <div>
              <Text type="secondary">可用模型: </Text>
              {currentProvider.models.map(model => (
                <Tag key={model}>{model}</Tag>
              ))}
            </div>
          </Space>
        </Card>
      )}
    </div>
  )
}
