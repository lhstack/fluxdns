<template>
  <div class="dashboard">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-left">
        <h1>仪表盘</h1>
        <p class="subtitle">系统运行状态概览</p>
      </div>
      <div class="header-right">
        <el-tag :type="stats.status === 'running' ? 'success' : 'danger'" size="large" effect="dark">
          <el-icon class="status-icon"><CircleCheck v-if="stats.status === 'running'" /><CircleClose v-else /></el-icon>
          {{ stats.status === 'running' ? '运行中' : '异常' }}
        </el-tag>
      </div>
    </div>

    <!-- 核心指标 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            <el-icon><DataAnalysis /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ stats.query?.total_queries || 0 }}</span>
            <span class="stat-label">总查询数</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);">
            <el-icon><TrendCharts /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ ((stats.cache?.hit_rate || 0) * 100).toFixed(1) }}%</span>
            <span class="stat-label">缓存命中率</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);">
            <el-icon><Coin /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ stats.cache?.entries || 0 }}</span>
            <span class="stat-label">缓存条目</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);">
            <el-icon><Timer /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ formatUptime(stats.uptime_seconds || 0) }}</span>
            <span class="stat-label">运行时间</span>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 详细信息卡片 -->
    <el-row :gutter="20" class="detail-row">
      <el-col :xs="24" :md="12">
        <el-card class="detail-card" shadow="never">
          <template #header>
            <div class="card-header">
              <el-icon class="card-icon"><Connection /></el-icon>
              <span>上游服务器</span>
            </div>
          </template>
          <div class="detail-list">
            <div class="detail-item">
              <span class="label">服务器总数</span>
              <span class="value">{{ stats.upstreams?.total || 0 }}</span>
            </div>
            <div class="detail-item">
              <span class="label">健康服务器</span>
              <span class="value success">{{ stats.upstreams?.healthy || 0 }}</span>
            </div>
            <div class="detail-item">
              <span class="label">查询策略</span>
              <el-tag type="primary" effect="plain" size="small">{{ strategyName }}</el-tag>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="12">
        <el-card class="detail-card" shadow="never">
          <template #header>
            <div class="card-header">
              <el-icon class="card-icon"><Calendar /></el-icon>
              <span>今日统计</span>
            </div>
          </template>
          <div class="detail-list">
            <div class="detail-item">
              <span class="label">今日查询</span>
              <span class="value">{{ stats.query?.queries_today || 0 }}</span>
            </div>
            <div class="detail-item">
              <span class="label">缓存命中</span>
              <span class="value success">{{ stats.query?.cache_hits || 0 }}</span>
            </div>
            <div class="detail-item">
              <span class="label">缓存上限</span>
              <span class="value">{{ stats.cache?.max_entries || 0 }}</span>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 快捷操作 -->
    <el-card class="quick-actions" shadow="never">
      <template #header>
        <div class="card-header">
          <el-icon class="card-icon"><Operation /></el-icon>
          <span>快捷操作</span>
        </div>
      </template>
      <el-row :gutter="16">
        <el-col :xs="12" :sm="6">
          <router-link to="/records" class="action-btn">
            <el-icon><Document /></el-icon>
            <span>DNS 记录</span>
          </router-link>
        </el-col>
        <el-col :xs="12" :sm="6">
          <router-link to="/upstreams" class="action-btn">
            <el-icon><Connection /></el-icon>
            <span>上游服务器</span>
          </router-link>
        </el-col>
        <el-col :xs="12" :sm="6">
          <router-link to="/cache" class="action-btn">
            <el-icon><Coin /></el-icon>
            <span>缓存管理</span>
          </router-link>
        </el-col>
        <el-col :xs="12" :sm="6">
          <router-link to="/logs" class="action-btn">
            <el-icon><List /></el-icon>
            <span>查询日志</span>
          </router-link>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { 
  CircleCheck, CircleClose, DataAnalysis, TrendCharts, Coin, Timer,
  Connection, Calendar, Operation, Document, List
} from '@element-plus/icons-vue'
import api from '../api'

interface Stats {
  status?: string
  uptime_seconds?: number
  cache?: {
    entries: number
    hits: number
    misses: number
    hit_rate: number
    default_ttl: number
    max_entries: number
  }
  query?: {
    total_queries: number
    cache_hits: number
    queries_today: number
  }
  upstreams?: {
    total: number
    healthy: number
  }
  strategy?: string
}

const stats = ref<Stats>({})
let refreshInterval: ReturnType<typeof setInterval> | null = null

const strategyName = computed(() => {
  const names: Record<string, string> = {
    'concurrent': '并发查询',
    'round_robin': '轮询',
    'random': '随机',
    'fastest': '最快响应'
  }
  return names[stats.value.strategy || ''] || stats.value.strategy || '-'
})

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  
  if (days > 0) return `${days}天${hours}时`
  if (hours > 0) return `${hours}时${minutes}分`
  return `${minutes}分钟`
}

async function fetchStats() {
  try {
    const response = await api.get('/api/status')
    stats.value = response.data
  } catch {
    // Handle error silently
  }
}

onMounted(() => {
  fetchStats()
  refreshInterval = setInterval(fetchStats, 30000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
})
</script>

<style scoped>
.dashboard {
  max-width: 1400px;
  margin: 0 auto;
}

/* 页面标题 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}

.header-left h1 {
  margin: 0 0 8px 0;
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.subtitle {
  margin: 0;
  font-size: 14px;
  color: #909399;
}

.status-icon {
  margin-right: 4px;
}

/* 统计卡片 */
.stats-row {
  margin-bottom: 24px;
}

.stat-card {
  background: #fff;
  border-radius: 12px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  transition: transform 0.3s, box-shadow 0.3s;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 28px;
}

.stat-info {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

/* 详细信息卡片 */
.detail-row {
  margin-bottom: 24px;
}

.detail-card {
  border-radius: 12px;
  border: none;
  height: 100%;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #303133;
}

.card-icon {
  font-size: 18px;
  color: #667eea;
}

.detail-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.detail-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 16px;
  border-bottom: 1px solid #f0f0f0;
}

.detail-item:last-child {
  padding-bottom: 0;
  border-bottom: none;
}

.detail-item .label {
  color: #909399;
  font-size: 14px;
}

.detail-item .value {
  font-weight: 600;
  color: #303133;
  font-size: 16px;
}

.detail-item .value.success {
  color: #67C23A;
}

/* 快捷操作 */
.quick-actions {
  border-radius: 12px;
  border: none;
}

.action-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px 16px;
  background: #f8f9fa;
  border-radius: 12px;
  text-decoration: none;
  color: #606266;
  transition: all 0.3s;
}

.action-btn:hover {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  transform: translateY(-2px);
}

.action-btn .el-icon {
  font-size: 28px;
}

.action-btn span {
  font-size: 14px;
  font-weight: 500;
}

/* 响应式 */
@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
    gap: 16px;
  }
  
  .stat-card {
    padding: 16px;
  }
  
  .stat-value {
    font-size: 22px;
  }
  
  .stat-icon {
    width: 48px;
    height: 48px;
    font-size: 24px;
  }
}
</style>
