import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { execSync } from 'child_process'

// 获取git信息
function getGitInfo() {
  try {
    const commitHash = execSync('git rev-parse --short HEAD').toString().trim()
    const commitTime = execSync('git log -1 --format=%ci').toString().trim()
    const commitMessage = execSync('git log -1 --format=%s').toString().trim()
    return { commitHash, commitTime, commitMessage }
  } catch {
    return { commitHash: 'unknown', commitTime: 'unknown', commitMessage: 'unknown' }
  }
}

const gitInfo = getGitInfo()

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    __GIT_COMMIT_HASH__: JSON.stringify(gitInfo.commitHash),
    __GIT_COMMIT_TIME__: JSON.stringify(gitInfo.commitTime),
    __GIT_COMMIT_MESSAGE__: JSON.stringify(gitInfo.commitMessage),
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
  },
  server: {
    port: 5173,
    host: true,
  },
})