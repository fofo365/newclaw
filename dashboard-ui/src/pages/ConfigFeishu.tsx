import { useEffect, useState } from 'react'
import { Card, Form, Input, Button, Space, message, Tag, Typography, Divider, Alert, Select, Switch } from 'antd'
import { SaveOutlined, ApiOutlined, CheckCircleOutlined, CloseCircleOutlined } from '@ant-design/icons'
import { getFeishuConfig, updateFeishuConfig, type FeishuConfig } from '../services/api'

const { Title, Text } = Typography
const { Option } = Select

export default function ConfigFeishu() {
  const [form] = Form.useForm()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [config, setConfig] = useState<FeishuConfig | null>(null)

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    setLoading(true)
    try {
      const res = await getFeishuConfig()
      setConfig(res.data)
      form.setFieldsValue({
        app_id: res.data.app_id || '',
        connection_mode: res.data.connection_mode || 'websocket',
        events_enabled: res.data.events_enabled,
      })
    } catch (error) {
      message.error('加载配置失败')
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async (values: Partial<FeishuConfig>) => {
    setSaving(true)
    try {
      await updateFeishuConfig(values)
      message.success('配置已保存')
      loadConfig()
    } catch (error) {
      message.error('保存失败')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div>
      <div className="page-header">
        <Title level={2}>飞书配置</Title>
        <Text type="secondary">配置飞书机器人连接参数</Text>
      </div>

      {/* 连接状态 */}
      <Card 
        title={
          <Space>
            <ApiOutlined />
            <span>连接状态</span>
          </Space>
        }
        style={{ marginBottom: 16 }}
        loading={loading}
      >
        <Space size="large">
          <div>
            <Text type="secondary">状态</Text>
            <br />
            {config?.configured ? (
              <Tag icon={<CheckCircleOutlined />} color="success">已配置</Tag>
            ) : (
              <Tag icon={<CloseCircleOutlined />} color="warning">未配置</Tag>
            )}
          </div>
          <div>
            <Text type="secondary">连接模式</Text>
            <br />
            <Tag color="blue">{config?.connection_mode || 'websocket'}</Tag>
          </div>
          <div>
            <Text type="secondary">事件订阅</Text>
            <br />
            <Tag color={config?.events_enabled ? 'green' : 'default'}>
              {config?.events_enabled ? '已启用' : '未启用'}
            </Tag>
          </div>
        </Space>
      </Card>

      {/* 配置表单 */}
      <Card title="配置参数" loading={loading}>
        <Alert
          message="安全提示"
          description="敏感信息（App Secret、Encrypt Key、Verification Token）应通过环境变量配置：FEISHU_APP_ID、FEISHU_APP_SECRET、FEISHU_ENCRYPT_KEY、FEISHU_VERIFICATION_TOKEN"
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
        />

        <Form
          form={form}
          layout="vertical"
          className="config-form"
          onFinish={handleSave}
        >
          <Form.Item name="app_id" label="App ID">
            <Input placeholder="cli_xxxxxxxxxxxx" disabled={!!config?.app_id} />
          </Form.Item>

          <Form.Item name="app_secret" label="App Secret">
            <Input.Password placeholder="敏感信息请通过环境变量配置" disabled />
          </Form.Item>

          <Divider />

          <Form.Item name="connection_mode" label="连接模式">
            <Select>
              <Option value="websocket">WebSocket（推荐）</Option>
              <Option value="longpoll">长轮询</Option>
            </Select>
          </Form.Item>

          <Form.Item 
            name="events_enabled" 
            label="启用事件订阅" 
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>

          <Divider />

          <Form.Item name="encrypt_key" label="Encrypt Key">
            <Input.Password placeholder="敏感信息请通过环境变量配置" disabled />
          </Form.Item>

          <Form.Item name="verification_token" label="Verification Token">
            <Input.Password placeholder="敏感信息请通过环境变量配置" disabled />
          </Form.Item>

          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" icon={<SaveOutlined />} loading={saving}>
                保存配置
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      {/* 环境变量说明 */}
      <Card title="环境变量配置" style={{ marginTop: 16 }}>
        <pre style={{ background: '#f5f5f5', padding: 16, borderRadius: 8, overflow: 'auto' }}>
{`# 飞书配置
FEISHU_APP_ID=cli_xxxxxxxxxxxx
FEISHU_APP_SECRET=xxxxxxxxxxxx
FEISHU_ENCRYPT_KEY=xxxxxxxxxxxx
FEISHU_VERIFICATION_TOKEN=xxxxxxxxxxxx

# 或在 config.toml 中配置
[feishu]
app_id = "cli_xxxxxxxxxxxx"
app_secret = "xxxxxxxxxxxx"
encrypt_key = "xxxxxxxxxxxx"
verification_token = "xxxxxxxxxxxx"`}
        </pre>
      </Card>
    </div>
  )
}
