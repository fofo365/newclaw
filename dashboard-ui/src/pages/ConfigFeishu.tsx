import { useEffect, useState } from 'react'
import { Card, Form, Input, Button, Space, message, Tag, Typography, Divider, Alert, Select, Switch, Modal } from 'antd'
import { SaveOutlined, ApiOutlined, CheckCircleOutlined, CloseCircleOutlined, EditOutlined } from '@ant-design/icons'
import { getFeishuConfig, updateFeishuConfig, type FeishuConfig } from '../services/api'

const { Title, Text, Paragraph } = Typography
const { Option } = Select
const { Password } = Input

export default function ConfigFeishu() {
  const [form] = Form.useForm()
  const [editForm] = Form.useForm()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [config, setConfig] = useState<FeishuConfig | null>(null)
  const [editModalVisible, setEditModalVisible] = useState(false)
  const [editingField, setEditingField] = useState<'app_id' | 'app_secret' | null>(null)

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

  const handleEditField = (field: 'app_id' | 'app_secret') => {
    setEditingField(field)
    editForm.setFieldsValue({
      [field]: config?.[field] || '',
    })
    setEditModalVisible(true)
  }

  const handleEditSave = async () => {
    try {
      const values = await editForm.validateFields()
      
      // 只发送当前编辑的字段，不发送其他掩码值
      const updatePayload: Partial<FeishuConfig> = {}
      if (editingField) {
        updatePayload[editingField] = values[editingField]
      }
      
      // 同时发送连接模式
      updatePayload.connection_mode = form.getFieldValue('connection_mode')
      
      await updateFeishuConfig(updatePayload)
      
      message.success('配置已更新')
      setEditModalVisible(false)
      loadConfig()
    } catch (error) {
      message.error('配置更新失败')
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
          description="敏感信息（App Secret、Encrypt Key、Verification Token）应通过环境变量配置。当前是临时实现，配置仅保存在内存中。"
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />

        <Form
          form={form}
          layout="vertical"
          className="config-form"
          onFinish={handleSave}
        >
          <Form.Item label="App ID">
            <Space.Compact style={{ width: '100%' }}>
              <Input
                value={config?.app_id || ''}
                placeholder="cli_xxxxxxxxxxxx"
                disabled
                style={{ flex: 1 }}
              />
              <Button
                icon={<EditOutlined />}
                onClick={() => handleEditField('app_id')}
              >
                编辑
              </Button>
            </Space.Compact>
          </Form.Item>

          <Form.Item label="App Secret">
            <Space.Compact style={{ width: '100%' }}>
              <Input.Password
                value="••••••••••••••••"
                disabled
                style={{ flex: 1 }}
              />
              <Button
                icon={<EditOutlined />}
                onClick={() => handleEditField('app_secret')}
              >
                编辑
              </Button>
            </Space.Compact>
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

      {/* 编辑模态框 */}
      <Modal
        title={`编辑 ${editingField === 'app_id' ? 'App ID' : 'App Secret'}`}
        open={editModalVisible}
        onCancel={() => setEditModalVisible(false)}
        onOk={handleEditSave}
        destroyOnClose
      >
        <Alert
          message="临时实现"
          description={
            <Paragraph>
              当前是临时实现，配置仅保存在内存中，重启服务后会丢失。
              要永久保存，请编辑配置文件：<Text code>/etc/newclaw/config.toml</Text>
            </Paragraph>
          }
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
        />

        <Form form={editForm} layout="vertical">
          <Form.Item
            name={editingField || 'app_id'}
            label={editingField === 'app_id' ? 'App ID' : 'App Secret'}
            rules={[{ required: true, message: '请输入值' }]}
          >
            {editingField === 'app_id' ? (
              <Input placeholder="cli_xxxxxxxxxxxx" />
            ) : (
              <Password placeholder="输入 App Secret" />
            )}
          </Form.Item>
        </Form>
      </Modal>
    </div>
  )
}
