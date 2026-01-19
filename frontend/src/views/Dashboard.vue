<template>
  <div class="dashboard">
    <!-- Hero 欢迎区 -->
    <div class="hero-section">
      <div class="hero-content">
        <div class="brand">
          <el-icon class="brand-icon"><Connection /></el-icon>
          <h1>FluxDNS</h1>
        </div>
        <p class="hero-description">
          一个功能完整的 DNS 代理服务，支持多种协议、AI 智能助手和现代化 Web 管理界面
        </p>
        <div class="hero-badges">
          <el-tag type="primary" effect="dark">Rust 驱动</el-tag>
          <el-tag type="success" effect="dark">高性能</el-tag>
          <el-tag type="warning" effect="dark">多协议</el-tag>
          <el-tag type="info" effect="dark">AI 智能</el-tag>
        </div>
      </div>
    </div>

    <!-- 核心功能卡片区 -->
    <div class="section-title">
      <h2>核心功能</h2>
      <p>探索 FluxDNS 的强大功能模块</p>
    </div>
    
    <el-row :gutter="20" class="feature-grid">
      <el-col :xs="24" :sm="12" :lg="8" v-for="feature in features" :key="feature.title">
        <el-card 
          class="feature-card" 
          shadow="hover"
          @click="navigateTo(feature.path)"
        >
          <div class="feature-icon" :style="{ background: feature.gradient }">
            <el-icon :size="28"><component :is="feature.icon" /></el-icon>
          </div>
          <h3>{{ feature.title }}</h3>
          <p>{{ feature.description }}</p>
          <div class="feature-link">
            <span>前往配置</span>
            <el-icon><ArrowRight /></el-icon>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 协议支持区 -->
    <div class="section-title">
      <h2>协议支持</h2>
      <p>支持多种现代 DNS 传输协议</p>
    </div>

    <el-card class="protocol-card" shadow="never">
      <el-table :data="protocols" style="width: 100%" stripe>
        <el-table-column prop="name" label="协议" width="180">
          <template #default="{ row }">
            <div class="protocol-name">
              <el-icon :size="18" :color="row.color"><component :is="row.icon" /></el-icon>
              <span>{{ row.name }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="port" label="默认端口" width="120" />
        <el-table-column prop="secure" label="安全传输" width="120">
          <template #default="{ row }">
            <el-tag :type="row.secure ? 'success' : 'info'" size="small">
              {{ row.secure ? '✅ 加密' : '❌ 明文' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="说明" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === '已实现' ? 'success' : 'warning'" size="small">
              {{ row.status }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 快速链接区 -->
    <div class="section-title">
      <h2>快速操作</h2>
      <p>常用功能快捷入口</p>
    </div>

    <el-row :gutter="16" class="quick-links">
      <el-col :xs="12" :sm="6" v-for="link in quickLinks" :key="link.title">
        <el-button 
          class="quick-link-btn"
          :type="link.type"
          size="large"
          @click="navigateTo(link.path)"
        >
          <el-icon><component :is="link.icon" /></el-icon>
          {{ link.title }}
        </el-button>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { 
  Connection, 
  ArrowRight,
  Monitor,
  Upload,
  Document,
  Refresh,
  Box,
  ChatDotRound,
  Search,
  Tickets,
  Setting,
  Cpu
} from '@element-plus/icons-vue'

const router = useRouter()

const features = [
  {
    title: '多协议监听',
    description: '支持 UDP、DoT、DoH、DoQ 多种 DNS 协议，灵活配置各类监听器',
    icon: Monitor,
    path: '/listeners',
    gradient: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
  },
  {
    title: '上游服务器',
    description: '配置多个上游 DNS 服务器，支持并发、轮询、最快响应等策略',
    icon: Upload,
    path: '/upstreams',
    gradient: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)'
  },
  {
    title: 'DNS 记录',
    description: '自定义本地 DNS 记录，支持泛域名解析和多种记录类型',
    icon: Document,
    path: '/records',
    gradient: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)'
  },
  {
    title: '域名重写',
    description: '灵活的域名重写规则，支持精确匹配、通配符和正则表达式',
    icon: Refresh,
    path: '/rewrite',
    gradient: 'linear-gradient(135deg, #43e97b 0%, #38f9d7 100%)'
  },
  {
    title: '缓存管理',
    description: '智能 DNS 缓存系统，提升查询效率，支持手动清除和导出',
    icon: Box,
    path: '/cache',
    gradient: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)'
  },
  {
    title: 'AI 智能助手',
    description: 'AI 驱动的 DNS 诊断分析，提供智能配置建议和问题排查',
    icon: ChatDotRound,
    path: '/llm',
    gradient: 'linear-gradient(135deg, #a18cd1 0%, #fbc2eb 100%)'
  }
]

const protocols = [
  { 
    name: 'UDP DNS', 
    port: '53', 
    secure: false, 
    description: '传统 UDP DNS 协议，速度快但无加密',
    status: '已实现',
    icon: Cpu,
    color: '#909399'
  },
  { 
    name: 'DoT (DNS over TLS)', 
    port: '853', 
    secure: true, 
    description: '使用 TLS 加密的 DNS 查询，保护隐私',
    status: '已实现',
    icon: Monitor,
    color: '#67c23a'
  },
  { 
    name: 'DoH (DNS over HTTPS)', 
    port: '443', 
    secure: true, 
    description: '通过 HTTPS 传输 DNS 请求，穿透防火墙',
    status: '已实现',
    icon: Connection,
    color: '#409eff'
  },
  { 
    name: 'DoQ (DNS over QUIC)', 
    port: '853', 
    secure: true, 
    description: '基于 QUIC 协议，低延迟高性能',
    status: '已实现',
    icon: Upload,
    color: '#e6a23c'
  },
  { 
    name: 'DoH3 (DNS over HTTP/3)', 
    port: '443', 
    secure: true, 
    description: '结合 HTTP/3 和 QUIC 的最新技术',
    status: '开发中',
    icon: Refresh,
    color: '#f56c6c'
  }
]

const quickLinks = [
  { title: 'DNS 查询测试', icon: Search, path: '/query', type: 'primary' as const },
  { title: '查询日志', icon: Tickets, path: '/logs', type: 'success' as const },
  { title: '系统设置', icon: Setting, path: '/settings', type: 'warning' as const },
  { title: '服务监听', icon: Monitor, path: '/listeners', type: 'info' as const }
]

const navigateTo = (path: string) => {
  router.push(path)
}
</script>

<style scoped>
.dashboard {
  max-width: 1200px;
  margin: 0 auto;
  padding-bottom: 40px;
}

/* Hero 区域 */
.hero-section {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 16px;
  padding: 48px 40px;
  margin-bottom: 40px;
  color: white;
  text-align: center;
}

.brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 16px;
}

.brand-icon {
  font-size: 42px;
}

.brand h1 {
  margin: 0;
  font-size: 42px;
  font-weight: 700;
  letter-spacing: 2px;
}

.hero-description {
  font-size: 18px;
  opacity: 0.95;
  margin: 0 0 24px 0;
  max-width: 600px;
  margin-left: auto;
  margin-right: auto;
  line-height: 1.6;
}

.hero-badges {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
}

.hero-badges .el-tag {
  font-size: 14px;
  padding: 8px 16px;
  border-radius: 20px;
}

/* Section 标题 */
.section-title {
  margin-bottom: 24px;
}

.section-title h2 {
  margin: 0 0 8px 0;
  font-size: 22px;
  font-weight: 600;
  color: #303133;
}

.section-title p {
  margin: 0;
  font-size: 14px;
  color: #909399;
}

/* 功能卡片 */
.feature-grid {
  margin-bottom: 48px;
}

.feature-card {
  cursor: pointer;
  transition: all 0.3s ease;
  margin-bottom: 20px;
  border-radius: 12px;
  border: none;
}

.feature-card:hover {
  transform: translateY(-6px);
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.12);
}

.feature-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  margin-bottom: 16px;
}

.feature-card h3 {
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.feature-card p {
  margin: 0 0 16px 0;
  font-size: 14px;
  color: #606266;
  line-height: 1.6;
  min-height: 44px;
}

.feature-link {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 14px;
  color: #667eea;
  font-weight: 500;
}

/* 协议表格 */
.protocol-card {
  margin-bottom: 48px;
  border-radius: 12px;
}

.protocol-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}

/* 快速链接 */
.quick-links {
  margin-bottom: 20px;
}

.quick-link-btn {
  width: 100%;
  height: 56px;
  font-size: 15px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.quick-link-btn .el-icon {
  font-size: 20px;
}

/* 响应式适配 */
@media (max-width: 768px) {
  .hero-section {
    padding: 32px 20px;
  }
  
  .brand h1 {
    font-size: 32px;
  }
  
  .brand-icon {
    font-size: 32px;
  }
  
  .hero-description {
    font-size: 15px;
  }
}
</style>
