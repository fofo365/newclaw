import { useState } from 'react'
import { Card, Form, Input, Button, Alert, Typography } from 'antd'
import { LoginOutlined } from '@ant-design/icons'
import { useNavigate } from 'react-router-dom'
import { loginWithPairCode, type LoginResponse } from '../services/api'
import { useAuth } from '../contexts/AuthContext'

const { Title, Text } = Typography

export default function Login() {
  const [form] = Form.useForm()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const navigate = useNavigate()
  const { login } = useAuth()

  const handleSubmit = async (values: { pair_code: string }) => {
    setLoading(true)
    setError('')

    try {
      const res = await loginWithPairCode(values)
      const data: LoginResponse = res.data

      // 使用 AuthContext 的 login 方法，传递 expires_at
      login(data.token, data.session_id, data.expires_at)

      // 跳转到 Dashboard
      navigate('/dashboard')
    } catch (err: any) {
      setError(err.response?.data?.message || '登录失败，请检查配对码')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{
      minHeight: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    }}>
      <Card style={{ width: 400, boxShadow: '0 10px 40px rgba(0,0,0,0.1)' }}>
        <div style={{ textAlign: 'center', marginBottom: 24 }}>
          <Title level={2} style={{ margin: 0 }}>NewClaw Dashboard</Title>
          <Text type="secondary">请输入配对码登录</Text>
          <div style={{ marginTop: 8 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              v0.7.0 | 构建时间: 2026-03-16 01:59 CST
            </Text>
          </div>
        </div>

        {error && (
          <Alert
            message={error}
            type="error"
            style={{ marginBottom: 16 }}
            showIcon
          />
        )}

        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          autoComplete="off"
        >
          <Form.Item
            label="配对码"
            name="pair_code"
            rules={[
              { required: true, message: '请输入配对码' },
              { pattern: /^\d{6}$/, message: '配对码为 6 位数字' }
            ]}
          >
            <Input
              placeholder="输入 6 位数字配对码"
              maxLength={6}
              size="large"
              prefix={<LoginOutlined />}
            />
          </Form.Item>

          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              size="large"
              loading={loading}
              block
            >
              登录
            </Button>
          </Form.Item>
        </Form>

        <Alert
          message="获取配对码"
          description="在服务器上运行：curl http://localhost:3001/api/auth/paircode"
          type="info"
          showIcon
        />
      </Card>
    </div>
  )
}