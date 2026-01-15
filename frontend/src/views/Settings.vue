<template>
  <div class="settings">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-left">
        <h1>系统设置</h1>
        <p class="subtitle">配置查询策略，查看系统状态和健康检查</p>
      </div>
      <el-button type="primary" size="large" @click="refreshAll">
        <el-icon><Refresh /></el-icon>
        刷新全部
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            <el-icon><Timer /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ formatUptimeShort(status.uptime_seconds) }}</span>
            <span class="stat-label">运行时间</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);">
            <el-icon><DataAnalysis /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ status.query?.total_queries || 0 }}</span>
            <span class="stat-label">总查询数</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #f5576c 0%, #f093fb 100%);">
            <el-icon><Box /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ status.cache?.entries || 0 }}</span>
            <span class="stat-label">缓存条目</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);">
            <el-icon><Connection /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ status.upstreams?.healthy || 0 }}/{{ status.upstreams?.total || 0 }}</span>
            <span class="stat-label">健康上游</span>
          </div>
        </div>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <!-- 查询策略 -->
      <el-col :xs="24" :md="12">
        <el-card class="strategy-card" shadow="never">
          <template #header>
            <div class="card-title">
              <el-icon><Setting /></el-icon>
              <span>查询策略</span>
            </div>
          </template>
          <div v-loading="loadingStrategy">
            <div class="current-strategy">
              <span class="current-label">当前策略</span>
              <el-tag type="primary" size="large" effect="dark">
                {{ getStrategyLabel(currentStrategy.strategy) }}
              </el-tag>
            </div>
            <p class="strategy-desc">{{ currentStrategy.description }}</p>
            
            <el-divider />
            
            <div class="strategy-options">
              <el-radio-group v-model="selectedStrategy" class="strategy-radio-group">
                <div 
                  v-for="strategy in availableStrategies" 
                  :key="strategy.name"
                  class="strategy-item"
                  :class="{ 'is-selected': selectedStrategy === strategy.name }"
                  @click="selectedStrategy = strategy.name"
                >
                  <el-radio :value="strategy.name" />
                  <div class="strategy-content">
                    <span class="strategy-name">{{ getStrategyLabel(strategy.name) }}</span>
                    <span class="strategy-option-desc">{{ strategy.description }}</span>
                  </div>
                </div>
              </el-radio-group>
            </div>
            
            <el-button
              type="primary"
              size="large"
              @click="saveStrategy"
              :loading="savingStrategy"
              :disabled="selectedStrategy === currentStrategy.strategy"
              class="save-btn"
            >
              <el-icon><Check /></el-icon>
              保存策略
            </el-button>
          </div>
        </el-card>
      </el-col>

      <!-- 系统状态 -->
      <el-col :xs="24" :md="12">
        <el-card class="status-card" shadow="never">
          <template #header>
            <div class="card-header">
              <div class="card-title">
                <el-icon><Monitor /></el-icon>
                <span>系统状态</span>
              </div>
              <el-button type="primary" link @click="fetchStatus">
                <el-icon><Refresh /></el-icon>
                刷新
              </el-button>
            </div>
          </template>
          <div v-loading="loadingStatus" class="status-content">
            <div class="status-item">
              <span class="status-label">运行状态</span>
              <el-tag :type="status.status === 'running' ? 'success' : 'danger'" effect="dark">
                {{ status.status === 'running' ? '运行中' : '异常' }}
              </el-tag>
            </div>
            <div class="status-item">
              <span class="status-label">运行时间</span>
              <span class="status-value">{{ formatUptime(status.uptime_seconds) }}</span>
            </div>
            <div class="status-item">
              <span class="status-label">缓存命中率</span>
              <span class="status-value highlight">{{ ((status.cache?.hit_rate || 0) * 100).toFixed(1) }}%</span>
            </div>
            <div class="status-item">
              <span class="status-label">今日查询</span>
              <span class="status-value">{{ status.query?.queries_today || 0 }}</span>
            </div>
            <div class="status-item">
              <span class="status-label">上游服务器</span>
              <span class="status-value">
                <el-tag type="success" size="small">{{ status.upstreams?.healthy || 0 }} 健康</el-tag>
                <el-tag type="info" size="small" style="margin-left: 4px;">{{ status.upstreams?.total || 0 }} 总计</el-tag>
              </span>
            </div>
            <div class="status-item">
              <span class="status-label">查询策略</span>
              <span class="status-value">{{ getStrategyLabel(status.strategy) }}</span>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 健康检查 -->
    <el-card class="health-card" shadow="never">
      <template #header>
        <div class="card-header">
          <div class="card-title">
            <el-icon><FirstAidKit /></el-icon>
            <span>健康检查</span>
          </div>
          <el-button type="primary" link @click="fetchHealth">
            <el-icon><Refresh /></el-icon>
            检查
          </el-button>
        </div>
      </template>
      <div v-loading="loadingHealth">
        <el-row :gutter="24">
          <el-col :xs="24" :sm="8">
            <div class="health-item" :class="{ 'is-healthy': health.database }">
              <div class="health-icon" :style="{ background: health.database ? 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)' : 'linear-gradient(135deg, #f5576c 0%, #f093fb 100%)' }">
                <el-icon><Coin /></el-icon>
              </div>
              <div class="health-info">
                <span class="health-label">数据库</span>
                <span class="health-status" :class="{ 'is-ok': health.database }">
                  {{ health.database ? '正常' : '异常' }}
                </span>
              </div>
              <el-icon class="health-check-icon" :class="{ 'is-ok': health.database }">
                <CircleCheck v-if="health.database" />
                <CircleClose v-else />
              </el-icon>
            </div>
          </el-col>
          <el-col :xs="24" :sm="8">
            <div class="health-item" :class="{ 'is-healthy': health.cache }">
              <div class="health-icon" :style="{ background: health.cache ? 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)' : 'linear-gradient(135deg, #f5576c 0%, #f093fb 100%)' }">
                <el-icon><Box /></el-icon>
              </div>
              <div class="health-info">
                <span class="health-label">缓存</span>
                <span class="health-status" :class="{ 'is-ok': health.cache }">
                  {{ health.cache ? '正常' : '异常' }}
                </span>
              </div>
              <el-icon class="health-check-icon" :class="{ 'is-ok': health.cache }">
                <CircleCheck v-if="health.cache" />
                <CircleClose v-else />
              </el-icon>
            </div>
          </el-col>
          <el-col :xs="24" :sm="8">
            <div class="health-item" :class="{ 'is-healthy': health.upstreams }">
              <div class="health-icon" :style="{ background: health.upstreams ? 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)' : 'linear-gradient(135deg, #f5576c 0%, #f093fb 100%)' }">
                <el-icon><Connection /></el-icon>
              </div>
              <div class="health-info">
                <span class="health-label">上游服务器</span>
                <span class="health-status" :class="{ 'is-ok': health.upstreams }">
                  {{ health.upstreams ? '正常' : '无可用' }}
                </span>
              </div>
              <el-icon class="health-check-icon" :class="{ 'is-ok': health.upstreams }">
                <CircleCheck v-if="health.upstreams" />
                <CircleClose v-else />
              </el-icon>
            </div>
          </el-col>
        </el-row>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { 
  Refresh, Timer, DataAnalysis, Box, Connection, Setting, Check,
  Monitor, FirstAidKit, Coin, CircleCheck, CircleClose
} from '@element-plus/icons-vue'
import api from '../api'

interface Strategy {
  strategy: string
  description: string
}

interface StrategyInfo {
  name: string
  description: string
}

interface SystemStatus {
  status: string
  uptime_seconds: number
  cache: {
    entries: number
    hits: number
    misses: number
    hit_rate: number
    default_ttl: number
    max_entries: number
  }
  query: {
    total_queries: number
    cache_hits: number
    queries_today: number
  }
  upstreams: {
    total: number
    healthy: number
    servers: any[]
  }
  strategy: string
}

interface HealthCheck {
  status: string
  database: boolean
  cache: boolean
  upstreams: boolean
}

const currentStrategy = ref<Strategy>({
  strategy: '',
  description: ''
})
const selectedStrategy = ref('')
const availableStrategies = ref<StrategyInfo[]>([])
const loadingStrategy = ref(false)
const savingStrategy = ref(false)

const status = ref<SystemStatus>({
  status: '',
  uptime_seconds: 0,
  cache: { entries: 0, hits: 0, misses: 0, hit_rate: 0, default_ttl: 60, max_entries: 10000 },
  query: { total_queries: 0, cache_hits: 0, queries_today: 0 },
  upstreams: { total: 0, healthy: 0, servers: [] },
  strategy: ''
})
const loadingStatus = ref(false)

const health = ref<HealthCheck>({
  status: '',
  database: false,
  cache: false,
  upstreams: false
})
const loadingHealth = ref(false)

const strategyLabels: Record<string, string> = {
  concurrent: '并发查询',
  fastest: '最快响应',
  round_robin: '轮询',
  random: '随机'
}

function getStrategyLabel(strategy: string): string {
  return strategyLabels[strategy] || strategy
}

function formatUptime(seconds: number): string {
  if (!seconds) return '-'
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  const parts = []
  if (days > 0) parts.push(`${days}天`)
  if (hours > 0) parts.push(`${hours}小时`)
  if (minutes > 0) parts.push(`${minutes}分钟`)
  if (secs > 0 || parts.length === 0) parts.push(`${secs}秒`)

  return parts.join(' ')
}

function formatUptimeShort(seconds: number): string {
  if (!seconds) return '-'
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)

  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

function refreshAll() {
  fetchStrategy()
  fetchStatus()
  fetchHealth()
}

async function fetchStrategy() {
  loadingStrategy.value = true
  try {
    const [currentRes, availableRes] = await Promise.all([
      api.get('/api/strategy'),
      api.get('/api/strategy/available')
    ])
    currentStrategy.value = currentRes.data
    selectedStrategy.value = currentRes.data.strategy
    availableStrategies.value = availableRes.data.strategies
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取策略配置失败')
  } finally {
    loadingStrategy.value = false
  }
}

async function saveStrategy() {
  savingStrategy.value = true
  try {
    const response = await api.put('/api/strategy', { strategy: selectedStrategy.value })
    currentStrategy.value = response.data
    ElMessage.success('策略已更新')
    fetchStatus()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存策略失败')
  } finally {
    savingStrategy.value = false
  }
}

async function fetchStatus() {
  loadingStatus.value = true
  try {
    const response = await api.get('/api/status')
    status.value = response.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取系统状态失败')
  } finally {
    loadingStatus.value = false
  }
}

async function fetchHealth() {
  loadingHealth.value = true
  try {
    const response = await api.get('/api/status/health')
    health.value = response.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '健康检查失败')
  } finally {
    loadingHealth.value = false
  }
}

onMounted(() => {
  fetchStrategy()
  fetchStatus()
  fetchHealth()
})
</script>

<style scoped>
.settings {
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
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 24px;
}

.stat-info {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

/* 卡片通用样式 */
.strategy-card,
.status-card,
.health-card {
  border-radius: 12px;
  border: none;
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.card-title .el-icon {
  color: #667eea;
}

/* 策略卡片 */
.current-strategy {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.current-label {
  font-size: 14px;
  color: #909399;
}

.strategy-desc {
  font-size: 13px;
  color: #606266;
  margin: 0 0 16px 0;
  padding: 12px;
  background: #f8f9fa;
  border-radius: 8px;
}

.strategy-options {
  margin-bottom: 20px;
}

.strategy-radio-group {
  width: 100%;
}

.strategy-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  background: #f8f9fa;
  border-radius: 8px;
  margin-bottom: 12px;
  cursor: pointer;
  transition: all 0.3s;
  border: 2px solid transparent;
}

.strategy-item:hover {
  background: #f0f2f5;
}

.strategy-item.is-selected {
  background: #ecf5ff;
  border-color: #409eff;
}

.strategy-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.strategy-name {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.strategy-option-desc {
  font-size: 12px;
  color: #909399;
  line-height: 1.5;
}

.save-btn {
  width: 100%;
}

/* 状态卡片 */
.status-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.status-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: #f8f9fa;
  border-radius: 8px;
}

.status-label {
  font-size: 14px;
  color: #606266;
}

.status-value {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
}

.status-value.highlight {
  color: #667eea;
  font-size: 16px;
}

/* 健康检查卡片 */
.health-card {
  margin-top: 0;
}

.health-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: #f8f9fa;
  border-radius: 12px;
  transition: all 0.3s;
  border: 2px solid transparent;
}

.health-item.is-healthy {
  border-color: #67c23a;
  background: #f0f9eb;
}

.health-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 24px;
  flex-shrink: 0;
}

.health-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.health-label {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}

.health-status {
  font-size: 13px;
  color: #f56c6c;
}

.health-status.is-ok {
  color: #67c23a;
}

.health-check-icon {
  font-size: 24px;
  color: #f56c6c;
}

.health-check-icon.is-ok {
  color: #67c23a;
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
    font-size: 20px;
  }
  
  .health-item {
    margin-bottom: 12px;
  }
}
</style>
