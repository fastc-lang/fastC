import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const DEFAULT_CODE = `// Welcome to FastC Playground!
// A safe C-like language that compiles to C11.

fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    let result: i32 = fibonacci(10);
    // Result: 55
    return result;
}
`

export const usePlaygroundStore = defineStore('playground', () => {
  const tokenStorageKey = 'fastc-playground-token'

  function getAuthToken(): string | null {
    const url = new URL(window.location.href)
    const tokenFromQuery = url.searchParams.get('token')
    if (tokenFromQuery) {
      localStorage.setItem(tokenStorageKey, tokenFromQuery)
      return tokenFromQuery
    }
    return localStorage.getItem(tokenStorageKey)
  }

  function authHeaders(): Record<string, string> {
    const token = getAuthToken()
    if (!token) {
      return { 'Content-Type': 'application/json' }
    }
    return {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
      'x-fastc-token': token,
    }
  }

  function wsUrl(): string {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const token = getAuthToken()
    const tokenQuery = token ? `?token=${encodeURIComponent(token)}` : ''
    return `${protocol}//${window.location.host}/ws${tokenQuery}`
  }

  // State
  const code = ref(DEFAULT_CODE)
  const cCode = ref('')
  const terminalOutput = ref<string[]>([])
  const isCompiling = ref(false)
  const isRunning = ref(false)
  const error = ref<string | null>(null)
  const errorLine = ref<number | null>(null)
  const ws = ref<WebSocket | null>(null)
  const sessionId = ref<string | null>(null)
  const terminalHeight = ref(200)

  // Parse error line from error message (fallback)
  function parseErrorLine(message: string): number | null {
    // Try various patterns: "line X", "Line X:", ":X:", etc.
    const patterns = [
      /[Ll]ine\s+(\d+)/,
      /:(\d+):/,
      /at\s+(\d+)/,
      /\[(\d+)\]/,
    ]
    for (const pattern of patterns) {
      const match = message.match(pattern)
      if (match) {
        return parseInt(match[1], 10)
      }
    }
    return null
  }

  // Extract error info from API response
  function extractErrorInfo(errorData: any): { message: string; line: number | null } {
    if (!errorData) return { message: 'Unknown error', line: null }
    const message = errorData.message || 'Compilation failed'
    // Use line from API if available, otherwise try to parse from message
    const line = errorData.line ?? parseErrorLine(message)
    return { message, line }
  }

  // Actions
  async function compile() {
    isCompiling.value = true
    error.value = null
    errorLine.value = null

    try {
      const response = await fetch('/api/compile', {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify({ code: code.value, emit_header: false }),
      })

      const data = await response.json()

      if (data.success) {
        cCode.value = data.c_code
      } else {
        const errorInfo = extractErrorInfo(data.error)
        error.value = errorInfo.message
        errorLine.value = errorInfo.line
        cCode.value = ''
      }
    } catch (e) {
      error.value = `Error: ${e}`
    } finally {
      isCompiling.value = false
    }
  }

  async function run() {
    isRunning.value = true
    isCompiling.value = true
    error.value = null
    errorLine.value = null
    terminalOutput.value = []

    try {
      const response = await fetch('/api/run', {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify({ code: code.value }),
      })

      const data = await response.json()

      if (data.success) {
        sessionId.value = data.session_id
        cCode.value = data.c_code || ''

        // Subscribe to WebSocket for output
        if (ws.value && ws.value.readyState === WebSocket.OPEN) {
          ws.value.send(JSON.stringify({ type: 'subscribe', session_id: data.session_id }))
        }
      } else {
        const errorInfo = extractErrorInfo(data.error)
        error.value = errorInfo.message
        errorLine.value = errorInfo.line
        cCode.value = ''
        isRunning.value = false
      }
    } catch (e) {
      error.value = `Error: ${e}`
      isRunning.value = false
    } finally {
      isCompiling.value = false
    }
  }

  async function format() {
    try {
      const response = await fetch('/api/format', {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify({ code: code.value }),
      })

      const data = await response.json()

      if (data.success && data.formatted) {
        code.value = data.formatted
      }
    } catch (e) {
      console.error('Format error:', e)
    }
  }

  function connectWebSocket() {
    const socket = new WebSocket(wsUrl())

    socket.onopen = () => {
      console.log('WebSocket connected')
    }

    socket.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data)
        handleWsMessage(msg)
      } catch (e) {
        console.error('WebSocket message error:', e)
      }
    }

    socket.onclose = () => {
      console.log('WebSocket disconnected, reconnecting...')
      setTimeout(connectWebSocket, 1000)
    }

    socket.onerror = (e) => {
      console.error('WebSocket error:', e)
    }

    ws.value = socket
  }

  function handleWsMessage(msg: any) {
    switch (msg.type) {
      case 'compile':
        terminalOutput.value.push(`[${msg.stage}] ${msg.output}`)
        break
      case 'stdout':
        terminalOutput.value.push(msg.data)
        break
      case 'stderr':
        terminalOutput.value.push(`\x1b[31m${msg.data}\x1b[0m`)
        break
      case 'exit':
        terminalOutput.value.push(
          msg.code === 0
            ? `\x1b[32mProcess exited with code ${msg.code}\x1b[0m`
            : `\x1b[31mProcess exited with code ${msg.code}\x1b[0m`
        )
        isRunning.value = false
        break
      case 'error':
        terminalOutput.value.push(`\x1b[31mError: ${msg.message}\x1b[0m`)
        errorLine.value = parseErrorLine(msg.message)
        isRunning.value = false
        break
    }
  }

  function setTerminalHeight(height: number) {
    terminalHeight.value = Math.max(100, Math.min(600, height))
  }

  function clearError() {
    error.value = null
    errorLine.value = null
  }

  function clearTerminal() {
    terminalOutput.value = []
  }

  function updateCode(newCode: string) {
    code.value = newCode
  }

  return {
    // State
    code,
    cCode,
    terminalOutput,
    isCompiling,
    isRunning,
    error,
    errorLine,
    terminalHeight,

    // Actions
    compile,
    run,
    format,
    connectWebSocket,
    clearTerminal,
    updateCode,
    setTerminalHeight,
    clearError,
  }
})
