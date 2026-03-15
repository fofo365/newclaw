import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom'
import { AuthProvider, useAuth } from './contexts/AuthContext'
import Login from './pages/Login'
import MainLayout from './components/MainLayout'
import Dashboard from './pages/Dashboard'
import ConfigLLM from './pages/ConfigLLM'
import ConfigTools from './pages/ConfigTools'
import ConfigFeishu from './pages/ConfigFeishu'
import MonitorLogs from './pages/MonitorLogs'
import MonitorMetrics from './pages/MonitorMetrics'
import Chat from './pages/Chat'
import AdminUsers from './pages/AdminUsers'
import AdminApiKeys from './pages/AdminApiKeys'
import Tasks from './pages/Tasks'
import Memory from './pages/Memory'
import Audit from './pages/Audit'
import Watchdog from './pages/Watchdog'
import Strategy from './pages/Strategy'
import Sessions from './pages/Sessions'

// 路由保护组件
function ProtectedRoute() {
  const { isAuthenticated } = useAuth()
  return isAuthenticated ? <Outlet /> : <Navigate to="/login" replace />
}

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route element={<ProtectedRoute />}>
            <Route path="/" element={<MainLayout />}>
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="config">
                <Route path="llm" element={<ConfigLLM />} />
                <Route path="tools" element={<ConfigTools />} />
                <Route path="feishu" element={<ConfigFeishu />} />
              </Route>
              <Route path="monitor">
                <Route path="logs" element={<MonitorLogs />} />
                <Route path="metrics" element={<MonitorMetrics />} />
              </Route>
              <Route path="chat" element={<Chat />} />
              <Route path="admin">
                <Route path="users" element={<AdminUsers />} />
                <Route path="apikeys" element={<AdminApiKeys />} />
              </Route>
              {/* v0.7.0 新增页面 */}
              <Route path="tasks" element={<Tasks />} />
              <Route path="memory" element={<Memory />} />
              <Route path="audit" element={<Audit />} />
              <Route path="watchdog" element={<Watchdog />} />
              <Route path="strategy" element={<Strategy />} />
              <Route path="sessions" element={<Sessions />} />
            </Route>
          </Route>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}

export default App
