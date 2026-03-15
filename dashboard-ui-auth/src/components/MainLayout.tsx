import { useState } from 'react'
import { Outlet, useNavigate, useLocation } from 'react-router-dom'
import { Layout, Menu, theme } from 'antd'
import {
  DashboardOutlined,
  SettingOutlined,
  MonitorOutlined,
  MessageOutlined,
  UserOutlined,
  ToolOutlined,
  ApiOutlined,
  CloudOutlined,
  FileTextOutlined,
} from '@ant-design/icons'

const { Header, Sider, Content } = Layout

const menuItems = [
  {
    key: '/dashboard',
    icon: <DashboardOutlined />,
    label: '仪表盘',
  },
  {
    key: '/config',
    icon: <SettingOutlined />,
    label: '配置管理',
    children: [
      { key: '/config/llm', icon: <CloudOutlined />, label: 'LLM 配置' },
      { key: '/config/tools', icon: <ToolOutlined />, label: '工具配置' },
      { key: '/config/feishu', icon: <ApiOutlined />, label: '飞书配置' },
    ],
  },
  {
    key: '/monitor',
    icon: <MonitorOutlined />,
    label: '监控面板',
    children: [
      { key: '/monitor/logs', icon: <FileTextOutlined />, label: '日志查看' },
      { key: '/monitor/metrics', icon: <MonitorOutlined />, label: '性能指标' },
    ],
  },
  {
    key: '/chat',
    icon: <MessageOutlined />,
    label: '对话测试',
  },
  {
    key: '/admin',
    icon: <UserOutlined />,
    label: '系统管理',
    children: [
      { key: '/admin/users', icon: <UserOutlined />, label: '用户管理' },
      { key: '/admin/apikeys', icon: <ApiOutlined />, label: 'API Key' },
    ],
  },
]

export default function MainLayout() {
  const [collapsed, setCollapsed] = useState(false)
  const navigate = useNavigate()
  const location = useLocation()
  const {
    token: { colorBgContainer },
  } = theme.useToken()

  const handleMenuClick = ({ key }: { key: string }) => {
    navigate(key)
  }

  // 获取当前选中的菜单项
  const getSelectedKeys = () => {
    const path = location.pathname
    // 查找匹配的菜单项
    for (const item of menuItems) {
      if (item.key === path) return [path]
      if (item.children) {
        for (const child of item.children) {
          if (child.key === path) return [path]
        }
      }
    }
    return [path]
  }

  // 获取展开的菜单
  const getOpenKeys = () => {
    const path = location.pathname
    const parts = path.split('/').filter(Boolean)
    if (parts.length > 1) {
      return ['/' + parts[0]]
    }
    return []
  }

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider
        collapsible
        collapsed={collapsed}
        onCollapse={(value) => setCollapsed(value)}
        theme="dark"
      >
        <div style={{
          height: 32,
          margin: 16,
          background: 'rgba(255, 255, 255, 0.2)',
          borderRadius: 6,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'white',
          fontWeight: 'bold',
        }}>
          {collapsed ? '🦀' : '🦀 NewClaw'}
        </div>
        <Menu
          theme="dark"
          selectedKeys={getSelectedKeys()}
          defaultOpenKeys={getOpenKeys()}
          mode="inline"
          items={menuItems}
          onClick={handleMenuClick}
        />
      </Sider>
      <Layout>
        <Header style={{
          padding: '0 24px',
          background: colorBgContainer,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}>
          <h2 style={{ margin: 0 }}>NewClaw Dashboard v0.4.0</h2>
          <span style={{ color: '#666' }}>飞书 AI Agent 管理平台</span>
        </Header>
        <Content style={{ margin: '24px 16px', padding: 24, background: colorBgContainer, minHeight: 280, borderRadius: 8 }}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  )
}
