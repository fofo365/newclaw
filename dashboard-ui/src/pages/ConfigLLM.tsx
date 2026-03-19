import { useEffect, useState } from 'react'
import { Card, Form, Select, InputNumber, Input, Button, Space, message, Tag, Typography, Divider, Alert, Modal } from 'antd'
import { SaveOutlined, ReloadOutlined, KeyOutlined } from '@ant-design/icons'
import { getLLMConfig, updateLLMConfig, type LLMConfig, type ProviderInfo } from '../services/api'

const { Title, Text, Paragraph } = Typography
const { Option } = Select
const { Password } = Input

export default function ConfigLLM() {
  const [form] = Form.useForm()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [providers, setProviders] = useState<ProviderInfo[]>([])
  const [selectedProvider, setSelectedProvider] = useState<string>('')
  const [apiKeyModalVisible, setApiKeyModalVisible] = useState(false)
  const [apiKeyForm] = Form.useForm()

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

  const handleApiKeyUpdate = async (values: { api_key: string; base_url?: string }) => {
    try {
      // 调用 API 更新 API Key
      const response = await fetch(`/api/config/apikeys/${selectedProvider}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(values),
      })
      
      if (response.ok) {
        message.success('API Key 已更新（注意：临时版本仅内存生效）')
        setApiKeyModalVisible(false)
        apiKeyForm.resetFields()
        loadConfig() // 重新加载配置
      } else {
        message.error('API Key 更新失败')
      }
    } catch (error) {
      message.error('API Key 更新失败')
    }
  }

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
          description="可以通过环境变量配置（推荐），或者在下方点击按钮配置。设置 OPENAI_API_KEY、ANTHROPIC_API_KEY 或 GLM_API_KEY 环境变量。"
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
        
        {selectedProvider && (
          <div style={{ marginTop: 16, textAlign: 'center' }}>
            <Button
              type="primary"
              icon={<KeyOutlined />}
              onClick={() => setApiKeyModalVisible(true)}
            >
              配置 {currentProvider?.display_name} API Key
            </Button>
          </div>
        )}
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

      {/* API Key 配置模态框 */}
      <Modal
        title={`配置 ${currentProvider?.display_name} API Key`}
        open={apiKeyModalVisible}
        onCancel={() => setApiKeyModalVisible(false)}
        footer={null}
        destroyOnClose
      >
        <Alert
          message="安全提示"
          description={
            <div>
              <Paragraph>
                当前是临时实现，API Key 仅保存在内存中，重启服务后会丢失。
              </Paragraph>
              <Paragraph>
                要永久保存 API Key，请编辑配置文件：<Text code>/etc/newclaw/config.toml</Text>
              </Paragraph>
            </div>
          }
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />
        
        <Form
          form={apiKeyForm}
          layout="vertical"
          onFinish={handleApiKeyUpdate}
        >
          <Form.Item
            name="api_key"
            label="API Key"
            rules={[{ required: true, message: '请输入 API Key' }]}
          >
            <Password placeholder="输入 API Key" />
          </Form.Item>
          
          <Form.Item
            name="base_url"
            label="Base URL (可选)"
            tooltip="自定义 API 端点，通常不需要填写"
          >
            <Input placeholder="https://api.example.com" />
          </Form.Item>
          
          <Form.Item>
            <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
              <Button onClick={() => setApiKeyModalVisible(false)}>
                取消
              </Button>
              <Button type="primary" htmlType="submit">
                保存 API Key
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  )
}
