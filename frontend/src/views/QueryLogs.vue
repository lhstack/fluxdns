<template>
  <div class="query-logs">
    <h1>查询日志</h1>

    <!-- Statistics -->
    <el-row :gutter="20" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>总查询数</template>
          <div class="stat-value">{{ stats.total_queries }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>今日查询</template>
          <div class="stat-value">{{ stats.queries_today }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>缓存命中</template>
          <div class="stat-value">{{ stats.cache_hits }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>命中率</template>
          <div class="stat-value">{{ (stats.cache_hit_rate * 100).toFixed(1) }}%</div>
        </el-card>
      </el-col>
    </el-row>

    <!-- Filters -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="filters">
        <el-form-item label="域名">
          <el-input
            v-model="filters.query_name"
            placeholder="搜索域名"
            clearable
            @clear="fetchLogs"
          />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="filters.query_type" placeholder="全部" clearable style="width: 120px">
            <el-option
              v-for="type in recordTypes"
              :key="type"
              :label="type"
              :value="type"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="客户端 IP">
          <el-input
            v-model="filters.client_ip"
            placeholder="搜索 IP"
            clearable
            @clear="fetchLogs"
          />
        </el-form-item>
        <el-form-item label="缓存命中">
          <el-select v-model="filters.cache_hit" placeholder="全部" clearable style="width: 120px">
            <el-option label="是" :value="true" />
            <el-option label="否" :value="false" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchLogs">搜索</el-button>
          <el-button @click="resetFilters">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- Logs Table -->
    <el-card>
      <el-table :data="logs" v-loading="loading" stripe>
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="query_name" label="查询域名" min-width="200" />
        <el-table-column prop="query_type" label="类型" width="80">
          <template #default="{ row }">
            <el-tag size="small">{{ row.query_type }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="client_ip" label="客户端 IP" width="140" />
        <el-table-column prop="response_code" label="响应码" width="120">
          <template #default="{ row }">
            <el-tag :type="getResponseCodeType(row.response_code)" size="small">
              {{ row.response_code || '-' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="response_time" label="响应时间" width="100">
          <template #default="{ row }">
            {{ row.response_time ? `${row.response_time}ms` : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="cache_hit" label="缓存" width="80">
          <template #default="{ row }">
            <el-tag :type="row.cache_hit ? 'success' : 'info'" size="small">
              {{ row.cache_hit ? '命中' : '未命中' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="upstream_used" label="上游服务器" width="120">
          <template #default="{ row }">
            {{ row.upstream_used || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="时间" width="180">
          <template #default="{ row }">
            {{ formatTime(row.created_at) }}
          </template>
        </el-table-column>
      </el-table>

      <!-- Pagination -->
      <div class="pagination-container">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import api from '../api'

interface QueryLog {
  id: number
  query_name: string
  query_type: string
  client_ip: string
  response_code: string | null
  response_time: number | null
  cache_hit: boolean
  upstream_used: string | null
  created_at: string
}

interface QueryStats {
  total_queries: number
  cache_hits: number
  queries_today: number
  cache_hit_rate: number
}

const recordTypes = ['A', 'AAAA', 'CNAME', 'MX', 'TXT', 'PTR', 'NS', 'SOA', 'SRV']

const logs = ref<QueryLog[]>([])
const loading = ref(false)
const total = ref(0)
const currentPage = ref(1)
const pageSize = ref(20)

const stats = ref<QueryStats>({
  total_queries: 0,
  cache_hits: 0,
  queries_today: 0,
  cache_hit_rate: 0
})

const filters = reactive({
  query_name: '',
  query_type: null as string | null,
  client_ip: '',
  cache_hit: null as boolean | null
})

const offset = computed(() => (currentPage.value - 1) * pageSize.value)

function getResponseCodeType(code: string | null): string {
  if (!code) return 'info'
  if (code === 'NOERROR') return 'success'
  if (code === 'NXDOMAIN') return 'warning'
  return 'danger'
}

function formatTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

async function fetchLogs() {
  loading.value = true
  try {
    const params: Record<string, any> = {
      limit: pageSize.value,
      offset: offset.value
    }
    
    if (filters.query_name) params.query_name = filters.query_name
    if (filters.query_type) params.query_type = filters.query_type
    if (filters.client_ip) params.client_ip = filters.client_ip
    if (filters.cache_hit !== null) params.cache_hit = filters.cache_hit

    const response = await api.get('/api/logs', { params })
    logs.value = response.data.data
    total.value = response.data.total
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取日志失败')
  } finally {
    loading.value = false
  }
}

async function fetchStats() {
  try {
    const response = await api.get('/api/logs/stats')
    stats.value = response.data
  } catch {
    // Silently fail for stats
  }
}

function resetFilters() {
  filters.query_name = ''
  filters.query_type = null
  filters.client_ip = ''
  filters.cache_hit = null
  currentPage.value = 1
  fetchLogs()
}

function handleSizeChange() {
  currentPage.value = 1
  fetchLogs()
}

function handlePageChange() {
  fetchLogs()
}

onMounted(() => {
  fetchLogs()
  fetchStats()
})
</script>

<style scoped>
.query-logs {
  padding: 20px;
}

.query-logs h1 {
  margin-bottom: 20px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-value {
  font-size: 28px;
  font-weight: bold;
  color: #409EFF;
  text-align: center;
}

.filter-card {
  margin-bottom: 20px;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
