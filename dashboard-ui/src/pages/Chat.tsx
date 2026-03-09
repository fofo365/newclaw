import { useEffect, useState, useRef } from 'react'
import { Card, Row, Col, List, Input, Button, Space, Typography, Tag, Empty, message, Drawer } from 'antd'
import { SendOutlined, PlusOutlined } from '@ant-design/icons'
import { listSessions, createSession, getSession, sendMessage, type ChatSession, type ChatMessage } from '../services/api'
import dayjs from 'dayjs'

const { Text, Paragraph } = Typography
const { TextArea } = Input

export default function Chat() {
  const [sessions, setSessions] = useState<ChatSession[]>([])
  const [currentSession, setCurrentSession] = useState<ChatSession | null>(null)
  const [input, setInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [sending, setSending] = useState(false)
  const [debugVisible, setDebugVisible] = useState(false)
  const [selectedMessage, setSelectedMessage] = useState<ChatMessage | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadSessions()
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [currentSession?.messages])

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const loadSessions = async () => {
    setLoading(true)
    try {
      const res = await listSessions()
      setSessions(res.data.sessions)
      if (res.data.sessions.length > 0 && !currentSession) {
        setCurrentSession(res.data.sessions[0])
      }
    } catch (error) {
      message.error('加载会话失败')
    } finally {
      setLoading(false)
    }
  }

  const handleCreateSession = async () => {
    try {
      const res = await createSession({ title: '新对话' })
      setSessions([res.data, ...sessions])
      setCurrentSession(res.data)
      message.success('已创建新会话')
    } catch (error) {
      message.error('创建会话失败')
    }
  }

  const handleSelectSession = async (id: string) => {
    try {
      const res = await getSession(id)
      setCurrentSession(res.data)
    } catch (error) {
      message.error('加载会话失败')
    }
  }

  const handleSend = async () => {
    if (!input.trim() || !currentSession) return

    const content = input.trim()
    setInput('')
    setSending(true)

    try {
      const res = await sendMessage(currentSession.id, content)
      // 更新当前会话的消息列表
      setCurrentSession(prev => prev ? {
        ...prev,
        messages: [...prev.messages, res.data],
        updated_at: new Date().toISOString(),
      } : null)
    } catch (error) {
      message.error('发送失败')
    } finally {
      setSending(false)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const showDebug = (msg: ChatMessage) => {
    setSelectedMessage(msg)
    setDebugVisible(true)
  }

  return (
    <div style={{ height: 'calc(100vh - 180px)' }}>
      <Row gutter={16} style={{ height: '100%' }}>
        {/* 会话列表 */}
        <Col span={6} style={{ height: '100%' }}>
          <Card
            title="会话列表"
            extra={
              <Button type="primary" icon={<PlusOutlined />} size="small" onClick={handleCreateSession}>
                新建
              </Button>
            }
            style={{ height: '100%' }}
            bodyStyle={{ padding: 0, height: 'calc(100% - 57px)', overflow: 'auto' }}
          >
            <List
              dataSource={sessions}
              loading={loading}
              renderItem={(session) => (
                <List.Item
                  onClick={() => handleSelectSession(session.id)}
                  style={{
                    padding: '12px 16px',
                    cursor: 'pointer',
                    background: currentSession?.id === session.id ? '#e6f7ff' : undefined,
                  }}
                >
                  <List.Item.Meta
                    title={session.title}
                    description={
                      <Space>
                        <Text type="secondary" style={{ fontSize: 12 }}>
                          {dayjs(session.updated_at).format('MM-DD HH:mm')}
                        </Text>
                        <Tag>{session.messages.length} 条</Tag>
                      </Space>
                    }
                  />
                </List.Item>
              )}
            />
          </Card>
        </Col>

        {/* 聊天窗口 */}
        <Col span={18} style={{ height: '100%' }}>
          <Card
            title={currentSession?.title || '选择或创建会话'}
            style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
            bodyStyle={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 0 }}
          >
            {currentSession ? (
              <>
                {/* 消息区域 */}
                <div className="chat-messages" style={{ flex: 1, overflow: 'auto' }}>
                  {currentSession.messages.length === 0 ? (
                    <Empty description="开始对话吧" style={{ marginTop: 100 }} />
                  ) : (
                    currentSession.messages.map((msg) => (
                      <div
                        key={msg.id}
                        className={`message ${msg.role}`}
                        onClick={() => showDebug(msg)}
                        style={{ cursor: 'pointer' }}
                      >
                        <Space style={{ marginBottom: 4 }}>
                          <Tag color={msg.role === 'user' ? 'blue' : 'green'}>
                            {msg.role === 'user' ? '用户' : '助手'}
                          </Tag>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            {dayjs(msg.timestamp).format('HH:mm:ss')}
                          </Text>
                          {msg.tokens && (
                            <Tag>{msg.tokens.total} tokens</Tag>
                          )}
                        </Space>
                        <Paragraph style={{ margin: 0, whiteSpace: 'pre-wrap' }}>
                          {msg.content}
                        </Paragraph>
                      </div>
                    ))
                  )}
                  <div ref={messagesEndRef} />
                </div>

                {/* 输入区域 */}
                <div style={{ padding: 16, borderTop: '1px solid #f0f0f0' }}>
                  <Space.Compact style={{ width: '100%' }}>
                    <TextArea
                      value={input}
                      onChange={(e) => setInput(e.target.value)}
                      onKeyPress={handleKeyPress}
                      placeholder="输入消息... (Enter 发送, Shift+Enter 换行)"
                      autoSize={{ minRows: 1, maxRows: 4 }}
                      style={{ flex: 1 }}
                    />
                    <Button
                      type="primary"
                      icon={<SendOutlined />}
                      loading={sending}
                      onClick={handleSend}
                      disabled={!input.trim()}
                    >
                      发送
                    </Button>
                  </Space.Compact>
                </div>
              </>
            ) : (
              <Empty description="选择或创建一个会话开始对话" style={{ margin: 'auto' }} />
            )}
          </Card>
        </Col>
      </Row>

      {/* 调试抽屉 */}
      <Drawer
        title="消息详情"
        placement="right"
        open={debugVisible}
        onClose={() => setDebugVisible(false)}
        width={500}
      >
        {selectedMessage && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <Text strong>角色</Text>
              <br />
              <Tag color={selectedMessage.role === 'user' ? 'blue' : 'green'}>
                {selectedMessage.role}
              </Tag>
            </div>
            <div>
              <Text strong>时间</Text>
              <br />
              <Text>{dayjs(selectedMessage.timestamp).format('YYYY-MM-DD HH:mm:ss.SSS')}</Text>
            </div>
            {selectedMessage.tokens && (
              <div>
                <Text strong>Token 使用</Text>
                <br />
                <Space>
                  <Tag>输入: {selectedMessage.tokens.input}</Tag>
                  <Tag>输出: {selectedMessage.tokens.output}</Tag>
                  <Tag color="blue">总计: {selectedMessage.tokens.total}</Tag>
                </Space>
              </div>
            )}
            <div>
              <Text strong>内容</Text>
              <br />
              <Paragraph style={{ background: '#f5f5f5', padding: 12, borderRadius: 4, whiteSpace: 'pre-wrap' }}>
                {selectedMessage.content}
              </Paragraph>
            </div>
            <div>
              <Text strong>元数据</Text>
              <br />
              <pre style={{ background: '#f5f5f5', padding: 12, borderRadius: 4, fontSize: 12, overflow: 'auto' }}>
                {JSON.stringify(selectedMessage.metadata, null, 2)}
              </pre>
            </div>
          </Space>
        )}
      </Drawer>
    </div>
  )
}
