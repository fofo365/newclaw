import { createContext, useContext, useState, useEffect, ReactNode } from 'react'

interface AuthContextType {
  isAuthenticated: boolean
  token: string | null
  login: (token: string, sessionId: string) => void
  logout: () => void
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(null)

  useEffect(() => {
    // 从 localStorage 读取 token
    const savedToken = localStorage.getItem('dashboard_token')
    if (savedToken) {
      setToken(savedToken)
    }
  }, [])

  const login = (newToken: string, sessionId: string) => {
    localStorage.setItem('dashboard_token', newToken)
    localStorage.setItem('dashboard_session', sessionId)
    setToken(newToken)
  }

  const logout = () => {
    localStorage.removeItem('dashboard_token')
    localStorage.removeItem('dashboard_session')
    setToken(null)
    window.location.href = '/login'
  }

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