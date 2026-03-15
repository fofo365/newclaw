import { createContext, useContext, useState, useEffect, ReactNode } from 'react'

interface AuthContextType {
  isAuthenticated: boolean
  token: string | null
  login: (token: string, sessionId: string, expiresAt?: string) => void
  logout: () => void
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(null)
  const [expiresAt, setExpiresAt] = useState<string | null>(null)

  useEffect(() => {
    // 从 localStorage 读取 token 和过期时间
    const savedToken = localStorage.getItem('dashboard_token')
    const savedExpiresAt = localStorage.getItem('dashboard_expires_at')
    
    if (savedToken) {
      // 检查 token 是否过期
      if (savedExpiresAt) {
        const expiry = new Date(savedExpiresAt)
        if (expiry > new Date()) {
          // Token 有效
          setToken(savedToken)
          setExpiresAt(savedExpiresAt)
        } else {
          // Token 已过期，清除并跳转登录
          localStorage.removeItem('dashboard_token')
          localStorage.removeItem('dashboard_expires_at')
          localStorage.removeItem('dashboard_session')
          setToken(null)
        }
      } else {
        // 没有过期时间，设置为有效（兼容旧版本）
        setToken(savedToken)
      }
    }
  }, [])

  const login = (newToken: string, sessionId: string, newExpiresAt?: string) => {
    localStorage.setItem('dashboard_token', newToken)
    localStorage.setItem('dashboard_session', sessionId)
    
    // 设置默认过期时间为 24 小时后
    const expires = newExpiresAt || new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString()
    localStorage.setItem('dashboard_expires_at', expires)
    
    setToken(newToken)
    setExpiresAt(expires)
  }

  const logout = () => {
    localStorage.removeItem('dashboard_token')
    localStorage.removeItem('dashboard_session')
    localStorage.removeItem('dashboard_expires_at')
    setToken(null)
    setExpiresAt(null)
    window.location.href = '/login'
  }

  // 定期检查 token 过期（每 30 秒）
  useEffect(() => {
    if (!token || !expiresAt) return

    const checkExpiration = () => {
      const expiry = new Date(expiresAt)
      if (expiry <= new Date()) {
        logout()
      }
    }

    const interval = setInterval(checkExpiration, 30000)
    return () => clearInterval(interval)
  }, [token, expiresAt])

  return (
    <AuthContext.Provider value={{ isAuthenticated: !!token, token, login, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}