<template>
  <!-- 浮动触发按钮 -->
  <div class="ai-assistant-trigger" @click="toggleChat" :class="{ hidden: isOpen }">
    <div class="trigger-effects">
      <div class="trigger-pulse"></div>
      <div class="trigger-ring"></div>
    </div>
    <div class="trigger-icon">
      <el-icon :size="28"><ChatDotRound /></el-icon>
    </div>
  </div>

  <!-- 聊天窗口 -->
  <Transition name="chat-pop">
    <div 
      v-if="isOpen" 
      class="ai-assistant-chat" 
      :class="{ fullscreen: isFullscreen }"
      :style="isFullscreen ? {} : chatStyle"
    >
      <!-- 流光边框背景 -->
      <div class="border-gradient"></div>
      
      <!-- 窗口内容容器 -->
      <div class="chat-container">
        <!-- 窗口头部 -->
        <div class="chat-header" @mousedown="startDrag">
          <div class="header-left">
            <div class="ai-avatar-container">
              <div class="ai-avatar">
                <el-icon :size="20"><Monitor /></el-icon>
              </div>
              <div class="status-dot" :class="{ online: isConfigured }"></div>
            </div>
            <div class="header-info">
              <span class="header-title">FluxDNS AI</span>
              <span class="header-subtitle">{{ isConfigured ? 'System Online' : 'Configuration Required' }}</span>
            </div>
          </div>
          <div class="header-actions" @mousedown.stop>
            <el-tooltip content="清空对话" placement="bottom" :show-after="500">
              <div class="action-btn" @click="clearMessages" :class="{ disabled: messages.length === 0 }">
                <el-icon><Delete /></el-icon>
              </div>
            </el-tooltip>
            <el-tooltip :content="isFullscreen ? '退出全屏' : '全屏模式'" placement="bottom" :show-after="500">
              <div class="action-btn" @click="toggleFullscreen">
                <el-icon>
                  <CopyDocument v-if="isFullscreen" />
                  <FullScreen v-else />
                </el-icon>
              </div>
            </el-tooltip>
            <div class="action-btn close" @click="toggleChat">
              <el-icon><Close /></el-icon>
            </div>
          </div>
        </div>

        <!-- 消息列表 -->
        <div class="chat-messages" ref="messagesContainer">
          <!-- 欢迎/空状态 -->
          <div v-if="messages.length === 0" class="welcome-screen">
            <div class="welcome-logo">
              <el-icon :size="48"><MagicStick /></el-icon>
              <div class="logo-ring"></div>
            </div>
            <h3>FluxDNS 智能助手</h3>
            <p>我是您的 DNS 系统管理专家。请选择以下指令或直接输入问题。</p>
            
            <!-- 分类快捷指令 -->
            <div class="quick-actions-container">
              <div class="action-tabs">
                <div 
                  v-for="cat in quickActionCategories" 
                  :key="cat.value"
                  class="action-tab"
                  :class="{ active: activeQuickTab === cat.value }"
                  @click="activeQuickTab = cat.value"
                >
                  {{ cat.label }}
                </div>
              </div>
              <div class="action-grid">
                <div 
                  v-for="action in currentQuickActions" 
                  :key="action.text" 
                  class="quick-action-card"
                  @click="sendQuickMessage(action.message)"
                >
                  <span class="action-icon">{{ action.icon }}</span>
                  <span class="action-text">{{ action.text }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 消息流 -->
          <div v-for="(msg, index) in messages" :key="index" class="message-row" :class="msg.role">
            <div class="message-wrapper">
              <div class="message-avatar">
                <el-icon v-if="msg.role === 'assistant'" :size="16"><Monitor /></el-icon>
                <el-icon v-else :size="16"><User /></el-icon>
              </div>
              <div class="message-bubble">
                <div class="message-content markdown-body" v-html="formatMessage(msg.content)"></div>
                
                <!-- Function 调用结果展示 -->
                <div v-if="msg.functionResults && msg.functionResults.length > 0" class="function-results">
                  <div class="function-header" @click="toggleFunctionDetails(index)">
                    <el-icon><Operation /></el-icon>
                    <span>已执行 {{ msg.functionResults.length }} 个系统操作</span>
                    <el-icon class="arrow" :class="{ rotated: msg.showDetails }"><ArrowDown /></el-icon>
                  </div>
                  <el-collapse-transition>
                    <div v-if="msg.showDetails" class="function-details">
                      <div v-for="(result, i) in msg.functionResults" :key="i" class="function-item">
                        <div class="func-name">{{ result.name }}</div>
                        <div class="func-code">
                          <pre>{{ JSON.stringify(result.data, null, 2) }}</pre>
                        </div>
                      </div>
                    </div>
                  </el-collapse-transition>
                </div>
              </div>
            </div>
          </div>

          <!-- 加载状态 -->
          <div v-if="isLoading" class="message-row assistant loading">
            <div class="message-wrapper">
              <div class="message-avatar">
                <el-icon :size="16"><Monitor /></el-icon>
              </div>
              <div class="message-bubble typing">
                <div class="typing-dot"></div>
                <div class="typing-dot"></div>
                <div class="typing-dot"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- 输入区域 -->
        <div class="chat-input-area">
          <div class="input-wrapper">
            <el-input
              v-model="inputMessage"
              type="textarea"
              :rows="1"
              :autosize="{ minRows: 1, maxRows: 4 }"
              :placeholder="isConfigured ? '输入您的指令...' : '请先在配置页启用 LLM'"
              :disabled="!isConfigured || isLoading"
              @keydown.enter.prevent="handleEnter"
              resize="none"
              class="custom-textarea"
            />
            <div class="input-actions" style="padding-bottom: 0;">
              <el-button 
                type="primary" 
                circle
                :disabled="!inputMessage.trim() || !isConfigured || isLoading"
                @click="sendMessage"
                class="send-btn"
              >
                <el-icon><Promotion /></el-icon>
              </el-button>
            </div>
          </div>
          <!-- 上下文标签 -->
          <div class="context-bar" v-if="currentContext">
            <el-tag size="small" effect="dark" type="info" class="context-tag">
              <el-icon><InfoFilled /></el-icon> Context: {{ currentContext }}
            </el-tag>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import {
  ChatDotRound, Monitor, Delete, Close, MagicStick, User, Operation, 
  Promotion, InfoFilled, FullScreen, CopyDocument, ArrowDown
} from '@element-plus/icons-vue'
import api from '../api'

// 类型定义
interface FunctionResult {
  name: string
  data: any
}

interface Message {
  role: 'user' | 'assistant'
  content: string
  functionResults?: FunctionResult[]
  showDetails?: boolean //用于控制函数结果折叠
}

interface QuickActionCategory {
  label: string
  value: string
  actions: { icon: string; text: string; message: string }[]
}

const route = useRoute()

// 状态
const isOpen = ref(false)
const isLoading = ref(false)
const isConfigured = ref(false)
const isFullscreen = ref(false)
const inputMessage = ref('')
const messages = ref<Message[]>([])
const messagesContainer = ref<HTMLElement>()
const activeQuickTab = ref('common')

// 拖拽相关
const position = ref({ x: 0, y: 0 })
const isDragging = ref(false)
const dragStart = ref({ x: 0, y: 0 })

const chatStyle = computed(() => ({
  transform: `translate(${position.value.x}px, ${position.value.y}px)`
}))

// 快捷指令分类
const quickActionCategories: QuickActionCategory[] = [
  {
    label: '常用',
    value: 'common',
    actions: [
      { icon: '📊', text: '系统状态', message: '查看当前系统资源占用和运行状态' },
      { icon: '🔍', text: '日志分析', message: '分析最近 24 小时的查询日志，找出异常' },
      { icon: '🚀', text: '性能概览', message: '当前 DNS 解析性能如何？' },
      { icon: '🧹', text: '清理缓存', message: '清空 DNS 缓存并报告释放的内存' }
    ]
  },
  {
    label: 'DNS',
    value: 'dns',
    actions: [
      { icon: '➕', text: '添加记录', message: '帮我添加一条 A 记录' },
      { icon: '📋', text: '列出记录', message: '列出所有 example.com 的记录' },
      { icon: '🔄', text: '重写规则', message: '查看当前的 DNS 重写规则' },
      { icon: '🔎', text: '查询测试', message: '测试 google.com 的解析结果' }
    ]
  },
  {
    label: '诊断',
    value: 'diag',
    actions: [
      { icon: '🩺', text: '上游健康', message: '检测所有上游服务器的健康状态' },
      { icon: '🔗', text: '链路追踪', message: '对 baidu.com 进行 DNS 解析链路追踪' },
      { icon: '🆚', text: '与 Google 对比', message: '对比当前 DNS 与 8.8.8.8 的解析结果' }
    ]
  },
  {
    label: '分析',
    value: 'analytics',
    actions: [
      { icon: '📈', text: '访问统计', message: '统计今日访问量最高的域名 Top 10' },
      { icon: '👥', text: '客户端分析', message: '分析最活跃的客户端 IP' },
      { icon: '🛡️', text: '安全扫描', message: '检测最近是否有 DNS 隧道攻击迹象' }
    ]
  }
]

const currentQuickActions = computed(() => {
  return quickActionCategories.find(c => c.value === activeQuickTab.value)?.actions || []
})

// 上下文感知
const currentContext = computed(() => {
  const contexts: Record<string, string> = {
    '/': '仪表盘 (Dashboard)',
    '/records': 'DNS 记录管理',
    '/rewrite': '重写规则管理',
    '/upstreams': '上游服务器管理',
    '/cache': '缓存管理',
    '/query': 'DNS 查询工具',
    '/logs': '查询日志',
    '/listeners': '监听器配置',
    '/settings': '系统设置',
    '/llm': 'AI 助手配置'
  }
  return contexts[route.path] || null
})

// 方法
function toggleChat() {
  isOpen.value = !isOpen.value
  if (isOpen.value) {
    checkConfiguration()
    scrollToBottom()
  }
}

function toggleFullscreen() {
  isFullscreen.value = !isFullscreen.value
  // 全屏切换后重置位置，或者保持位置逻辑，这里由 CSS 控制，重置拖拽位置可能更好
  if (isFullscreen.value) {
    // 记录全屏前的位置可能有用，但为了简化，全屏直接覆盖
  }
  scrollToBottom()
}

function startDrag(e: MouseEvent) {
  if (isFullscreen.value) return // 全屏不可拖动
  if ((e.target as HTMLElement).closest('.header-actions')) return // 点击按钮不拖动
  
  isDragging.value = true
  dragStart.value = { x: e.clientX - position.value.x, y: e.clientY - position.value.y }
  
  const onMouseMove = (e: MouseEvent) => {
    if (!isDragging.value) return
    position.value = {
      x: e.clientX - dragStart.value.x,
      y: e.clientY - dragStart.value.y
    }
  }
  
  const onMouseUp = () => {
    isDragging.value = false
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  }
  
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

async function checkConfiguration() {
  try {
    const { data } = await api.get('/api/llm/config')
    isConfigured.value = data.some((c: any) => c.enabled)
  } catch {
    isConfigured.value = false
  }
}

function formatMessage(content: string): string {
  if (!content) return ''
  // 增强的 Markdown 处理
  let html = content
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  
  // 代码块 ```code```
  html = html.replace(/```([\s\S]*?)```/g, '<pre><code>$1</code></pre>')
  
  // 行内代码 `code`
  html = html.replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>')
  
  // 加粗 **text**
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
  
  // 列表 - item
  html = html.replace(/^\s*-\s+(.*)$/gm, '<li>$1</li>')
  // 将连续的 li 包裹在 ul 中 (简单处理)
  // 这里正则可能不够完美，但对于简单的 LLM 输出足够
  html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
  
  // 换行
  html = html.replace(/\n/g, '<br>')
  
  return html
}

function handleEnter(e: KeyboardEvent) {
  if (!e.shiftKey) {
    sendMessage()
  }
}

async function sendMessage() {
  if (!inputMessage.value.trim() || !isConfigured.value || isLoading.value) return
  
  const userMessage = inputMessage.value.trim()
  inputMessage.value = ''
  
  messages.value.push({ role: 'user', content: userMessage })
  scrollToBottom()
  
  isLoading.value = true
  try {
    const context = currentContext.value ? `当前用户Focus在"${currentContext.value}"页面。` : ''
    const { data } = await api.post('/api/llm/chat', {
      message: userMessage,
      context
    })
    
    messages.value.push({
      role: 'assistant',
      content: data.reply,
      functionResults: data.functions_called?.length ? data.functions_called : undefined,
      showDetails: false
    })
  } catch (error: any) {
    const errorMsg = error.response?.data?.message || '连接服务失败，请检查后端日志。'
    messages.value.push({
      role: 'assistant',
      content: `❌ **错误**: ${errorMsg}`
    })
  } finally {
    isLoading.value = false
    scrollToBottom()
  }
}

function sendQuickMessage(message: string) {
  inputMessage.value = message
  sendMessage()
}

function clearMessages() {
  messages.value = []
}

function toggleFunctionDetails(index: number) {
  if (messages.value[index]) {
    messages.value[index].showDetails = !messages.value[index].showDetails
  }
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

onMounted(() => {
  checkConfiguration()
})
</script>

<style scoped>
/* 触发按钮容器 */
.ai-assistant-trigger {
  position: fixed;
  bottom: 24px;
  right: 24px;
  width: 60px;
  height: 60px;
  z-index: 2000;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.ai-assistant-trigger:hover {
  transform: scale(1.1);
}

.ai-assistant-trigger.hidden {
  transform: scale(0) rotate(180deg);
  opacity: 0;
  pointer-events: none;
}

.trigger-icon {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: linear-gradient(135deg, #00f2fe 0%, #4facfe 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  box-shadow: 0 8px 32px rgba(79, 172, 254, 0.5);
  position: relative;
  z-index: 2;
}

/* 脉冲动画 */
.trigger-pulse {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: rgba(79, 172, 254, 0.4);
  animation: pulse 2s infinite;
}

.trigger-ring {
  position: absolute;
  top: -4px;
  left: -4px;
  right: -4px;
  bottom: -4px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.3);
  opacity: 0.5;
  animation: spin 10s linear infinite;
}

@keyframes pulse {
  0% { transform: translate(-50%, -50%) scale(1); opacity: 0.6; }
  100% { transform: translate(-50%, -50%) scale(1.6); opacity: 0; }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 聊天窗口主容器 */
.ai-assistant-chat {
  position: fixed;
  bottom: 100px;
  right: 24px;
  width: 480px; /* 加宽 */
  height: 700px;
  max-height: calc(100vh - 120px);
  z-index: 2001;
  display: flex;
  flex-direction: column;
  border-radius: 20px;
  background: rgba(10, 20, 30, 0.85); /* 深色半透明背景 */
  backdrop-filter: blur(20px);
  box-shadow: 0 40px 80px rgba(0, 0, 0, 0.6), 
              0 0 0 1px rgba(255, 255, 255, 0.1);
  overflow: hidden;
  transition: width 0.3s ease, height 0.3s ease, bottom 0.3s ease, right 0.3s ease, transform 0s; /* transform 不过渡以避免拖拽延迟 */
}

/* 全屏模式 */
.ai-assistant-chat.fullscreen {
  width: 90vw !important;
  height: 90vh !important;
  bottom: 5vh !important;
  right: 5vw !important;
  transform: none !important;
  max-width: 1200px;
  max-height: none;
}

/* 流光边框 - 使用伪元素 */
.border-gradient {
  position: absolute;
  inset: 0;
  border-radius: 20px;
  padding: 1px; /* 边框宽度 */
  background: linear-gradient(135deg, rgba(255,255,255,0.1), rgba(255,255,255,0)); 
  -webkit-mask: 
     linear-gradient(#fff 0 0) content-box, 
     linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  pointer-events: none;
  z-index: 2;
}

.chat-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
  z-index: 3;
}

/* Header */
.chat-header {
  padding: 16px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  cursor: grab;
}

.chat-header:active {
  cursor: grabbing;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.ai-avatar-container {
  position: relative;
}

.ai-avatar {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: linear-gradient(135deg, #4facfe, #00f2fe);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  box-shadow: 0 4px 12px rgba(79, 172, 254, 0.3);
}

.status-dot {
  position: absolute;
  bottom: -2px;
  right: -2px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #666;
  border: 2px solid #0f172a;
}

.status-dot.online {
  background: #10b981;
  box-shadow: 0 0 8px #10b981;
}

.header-info {
  display: flex;
  flex-direction: column;
}

.header-title {
  font-weight: 700;
  color: #fff;
  font-size: 16px;
  letter-spacing: 0.5px;
}

.header-subtitle {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.6);
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.action-btn.close:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.action-btn.disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* Messages Area */
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 24px;
  scroll-behavior: smooth;
}

/* Scrollbar styling */
.chat-messages::-webkit-scrollbar {
  width: 4px;
}
.chat-messages::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

/* Welcome Screen */
.welcome-screen {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 40px;
  text-align: center;
}

.welcome-logo {
  position: relative;
  width: 80px;
  height: 80px;
  margin-bottom: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #4facfe;
}

.logo-ring {
  position: absolute;
  inset: 0;
  border-radius: 24px;
  border: 1px solid rgba(79, 172, 254, 0.3);
  transform: rotate(45deg);
}

.welcome-screen h3 {
  margin: 0;
  font-size: 20px;
  color: #fff;
  margin-bottom: 8px;
}

.welcome-screen p {
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
  margin-bottom: 32px;
  max-width: 80%;
}

/* Quick Actions */
.quick-actions-container {
  width: 100%;
}

.action-tabs {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 4px;
  margin-bottom: 16px;
  background: rgba(255, 255, 255, 0.05);
  padding: 4px;
  border-radius: 12px;
  align-self: center;
}

.action-tab {
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 13px;
  color: rgba(255, 255, 255, 0.6);
  cursor: pointer;
  transition: all 0.3s;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.action-tab:hover {
  color: #fff;
}

.action-tab.active {
  background: rgba(79, 172, 254, 0.2);
  color: #4facfe;
  font-weight: 600;
}

.action-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.quick-action-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
  padding: 12px;
  border-radius: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.2s;
  text-align: left;
}

.quick-action-card:hover {
  background: rgba(79, 172, 254, 0.1);
  border-color: rgba(79, 172, 254, 0.3);
  transform: translateY(-2px);
}

.action-icon {
  font-size: 18px;
}

.action-text {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.8);
}

/* Message Rows */
.message-row {
  display: flex;
  width: 100%;
}

.message-row.user {
  justify-content: flex-end;
}

.message-wrapper {
  display: flex;
  gap: 12px;
  max-width: 85%;
}

.message-row.user .message-wrapper {
  flex-direction: row-reverse;
}

.message-avatar {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.6);
  margin-top: 4px;
}

.message-row.assistant .message-avatar {
  background: linear-gradient(135deg, rgba(79, 172, 254, 0.2), rgba(0, 242, 254, 0.1));
  color: #4facfe;
  border: 1px solid rgba(79, 172, 254, 0.2);
}

.message-bubble {
  padding: 12px 16px;
  border-radius: 16px;
  font-size: 14px;
  line-height: 1.6;
  position: relative;
}

.message-row.assistant .message-bubble {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.05);
  color: rgba(255, 255, 255, 0.9);
  border-top-left-radius: 4px;
}

.message-row.user .message-bubble {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: #fff;
  border-top-right-radius: 4px;
  box-shadow: 0 4px 12px rgba(79, 172, 254, 0.3);
}

/* Typing Indicator */
.message-bubble.typing {
  display: flex;
  gap: 6px;
  padding: 16px;
  min-width: 60px;
  justify-content: center;
}

.typing-dot {
  width: 6px;
  height: 6px;
  background: rgba(255, 255, 255, 0.4);
  border-radius: 50%;
  animation: bounce 1.4s infinite ease-in-out both;
}

.typing-dot:nth-child(1) { animation-delay: -0.32s; }
.typing-dot:nth-child(2) { animation-delay: -0.16s; }

@keyframes bounce {
  0%, 80%, 100% { transform: scale(0); }
  40% { transform: scale(1); }
}

/* Input Area */
.chat-input-area {
  padding: 16px;
  background: rgba(0, 0, 0, 0.2);
  border-top: 1px solid rgba(255, 255, 255, 0.05);
}

.input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 16px;
  padding: 6px 8px 6px 16px;
  transition: all 0.2s;
}

.input-wrapper:focus-within {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(79, 172, 254, 0.3);
}

.custom-textarea :deep(.el-textarea__inner) {
  background: transparent !important;
  box-shadow: none !important;
  border: none !important;
  padding: 4px 0;
  color: white;
  min-height: 24px !important;
  line-height: 1.5;
}

.input-actions {
  display: flex;
  align-items: center;
}

.send-btn {
  background: linear-gradient(135deg, #4facfe, #00f2fe);
  border: none;
  width: 36px;
  height: 36px;
}

.send-btn.is-disabled {
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.3);
}

.context-bar {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}

.context-tag {
  background: rgba(255, 255, 255, 0.05);
  border: none;
  font-size: 11px;
}

/* Function Results */
.function-results {
  margin-top: 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  padding-top: 8px;
}

.function-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #4facfe;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
}

.function-header:hover {
  background: rgba(79, 172, 254, 0.1);
}

.arrow {
  margin-left: auto;
  transition: transform 0.3s;
}

.arrow.rotated {
  transform: rotate(180deg);
}

.function-details {
  margin-top: 8px;
}

.function-item {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 8px;
  padding: 10px;
  margin-bottom: 8px;
}

.func-name {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 4px;
  font-family: monospace;
}

.func-code pre {
  margin: 0;
  font-family: 'Fira Code', monospace;
  font-size: 12px;
  color: #a5b4fc;
  white-space: pre-wrap;
  word-break: break-all;
}

/* Animations */
.chat-pop-enter-active,
.chat-pop-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.chat-pop-enter-from,
.chat-pop-leave-to {
  opacity: 0;
  transform: scale(0.9) translateY(20px);
}

/* Mobile Responsiveness */
@media (max-width: 768px) {
  .ai-assistant-chat {
    width: 100% !important;
    right: 0 !important;
    bottom: 0 !important;
    height: 90vh !important;
    border-radius: 20px 20px 0 0;
    max-height: none;
    transform: none !important;
  }
  
  .ai-assistant-trigger {
    bottom: 20px;
    right: 20px;
  }
}
</style>
