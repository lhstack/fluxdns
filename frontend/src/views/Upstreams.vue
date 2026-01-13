<template>
  <div class="upstreams">
    <div class="page-header">
      <h1>上游服务器管理</h1>
      <el-button type="primary" @click="openCreateDialog">
        <el-icon><Plus /></el-icon>
        添加服务器
      </el-button>
    </div>

    <el-card>
      <el-table :data="servers" v-loading="loading" stripe>
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" label="名称" min-width="150" />
        <el-table-column prop="address" label="地址" min-width="250" />
        <el-table-column prop="protocol" label="协议" width="100">
          <template #default="{ row }">
            <el-tag :type="getProtocolTag(row.protocol)">
              {{ row.protocol.toUpperCase() }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="timeout" label="超时(ms)" width="100" />
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusTag(row)">
              {{ getStatusLabel(row) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="enabled" label="启用" width="80">
          <template #default="{ row }">
            <el-switch
              v-model="row.enabled"
              @change="toggleEnabled(row)"
            />
          </template>
        </el-table-column>
        <el-table-column label="统计" width="180">
          <template #default="{ row }">
            <div class="stats-cell">
              <span>查询: {{ getServerStats(row.id)?.queries || 0 }}</span>
              <span>失败: {{ getServerStats(row.id)?.failures || 0 }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link @click="openEditDialog(row)">
              编辑
            </el-button>
            <el-button type="danger" link @click="confirmDelete(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="pagination.page"
          v-model:page-size="pagination.pageSize"
          :page-sizes="[5, 10, 20, 50, 100]"
          :total="pagination.total"
          layout="total, sizes, prev, pager, next, jumper"
          :pager-count="5"
          prev-text="上一页"
          next-text="下一页"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑服务器' : '添加服务器'"
      width="500px"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="80px"
      >
        <el-form-item label="名称" prop="name">
          <el-input v-model="formData.name" placeholder="Cloudflare DNS" />
        </el-form-item>
        <el-form-item label="协议" prop="protocol">
          <el-select v-model="formData.protocol" placeholder="选择协议">
            <el-option label="UDP" value="udp" />
            <el-option label="DoT (DNS over TLS)" value="dot" />
            <el-option label="DoH (DNS over HTTPS)" value="doh" />
            <el-option label="DoQ (DNS over QUIC)" value="doq" />
            <el-option label="DoH3 (DNS over HTTP/3)" value="doh3" />
          </el-select>
        </el-form-item>
        <el-form-item label="地址" prop="address">
          <el-input
            v-model="formData.address"
            :placeholder="getAddressPlaceholder(formData.protocol)"
          />
        </el-form-item>
        <el-form-item label="超时" prop="timeout">
          <el-input-number
            v-model="formData.timeout"
            :min="100"
            :max="60000"
            :step="100"
          />
          <span class="unit-label">毫秒</span>
        </el-form-item>
        <el-form-item label="启用" prop="enabled">
          <el-switch v-model="formData.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitForm" :loading="submitting">
          {{ isEditing ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import api from '../api'

interface UpstreamServer {
  id: number
  name: string
  address: string
  protocol: string
  timeout: number
  enabled: boolean
  created_at: string
  updated_at: string
}

interface ServerStatus {
  id: number
  name: string
  address: string
  protocol: string
  enabled: boolean
  healthy: boolean
  queries: number
  failures: number
  avg_response_time_ms: number
}

const servers = ref<UpstreamServer[]>([])
const serverStatus = ref<Map<number, ServerStatus>>(new Map())
const loading = ref(false)
const dialogVisible = ref(false)
const isEditing = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editingId = ref<number | null>(null)
let statusInterval: ReturnType<typeof setInterval> | null = null

const pagination = reactive({
  page: 1,
  pageSize: 20,
  total: 0
})

const formData = reactive({
  name: '',
  address: '',
  protocol: 'udp',
  timeout: 5000,
  enabled: true
})

const formRules: FormRules = {
  name: [
    { required: true, message: '请输入服务器名称', trigger: 'blur' },
    { max: 100, message: '名称长度不能超过100个字符', trigger: 'blur' }
  ],
  protocol: [
    { required: true, message: '请选择协议', trigger: 'change' }
  ],
  address: [
    { required: true, message: '请输入服务器地址', trigger: 'blur' }
  ],
  timeout: [
    { required: true, message: '请输入超时时间', trigger: 'blur' }
  ]
}

function getProtocolTag(protocol: string): string {
  const tags: Record<string, string> = {
    udp: '',
    dot: 'success',
    doh: 'warning',
    doq: 'danger',
    doh3: 'info'
  }
  return tags[protocol] || ''
}

function getStatusTag(server: UpstreamServer): string {
  if (!server.enabled) return 'info'
  const status = serverStatus.value.get(server.id)
  if (!status) return 'info'
  return status.healthy ? 'success' : 'danger'
}

function getStatusLabel(server: UpstreamServer): string {
  if (!server.enabled) return '已禁用'
  const status = serverStatus.value.get(server.id)
  if (!status) return '未知'
  return status.healthy ? '健康' : '异常'
}

function getServerStats(id: number): ServerStatus | undefined {
  return serverStatus.value.get(id)
}

function getAddressPlaceholder(protocol: string): string {
  const placeholders: Record<string, string> = {
    udp: '8.8.8.8:53',
    dot: '1.1.1.1:853',
    doh: 'https://dns.google/dns-query',
    doq: 'dns.adguard-dns.com:853',
    doh3: 'https://dns.adguard-dns.com/dns-query'
  }
  return placeholders[protocol] || ''
}

async function fetchServers() {
  loading.value = true
  try {
    const response = await api.get('/api/upstreams', {
      params: {
        page: pagination.page,
        page_size: pagination.pageSize
      }
    })
    servers.value = response.data.data
    pagination.total = response.data.total
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取服务器列表失败')
  } finally {
    loading.value = false
  }
}

function handleSizeChange(size: number) {
  pagination.pageSize = size
  pagination.page = 1
  fetchServers()
}

function handlePageChange(page: number) {
  pagination.page = page
  fetchServers()
}

async function fetchStatus() {
  try {
    const response = await api.get('/api/upstreams/status')
    const statusMap = new Map<number, ServerStatus>()
    for (const status of response.data.data) {
      statusMap.set(status.id, status)
    }
    serverStatus.value = statusMap
  } catch {
    // Silently fail for status updates
  }
}

function resetForm() {
  formData.name = ''
  formData.address = ''
  formData.protocol = 'udp'
  formData.timeout = 5000
  formData.enabled = true
  editingId.value = null
}

function openCreateDialog() {
  isEditing.value = false
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(server: UpstreamServer) {
  isEditing.value = true
  editingId.value = server.id
  formData.name = server.name
  formData.address = server.address
  formData.protocol = server.protocol
  formData.timeout = server.timeout
  formData.enabled = server.enabled
  dialogVisible.value = true
}

async function submitForm() {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitting.value = true
    try {
      if (isEditing.value && editingId.value) {
        await api.put(`/api/upstreams/${editingId.value}`, formData)
        ElMessage.success('服务器更新成功')
      } else {
        await api.post('/api/upstreams', formData)
        ElMessage.success('服务器创建成功')
      }
      dialogVisible.value = false
      fetchServers()
      fetchStatus()
    } catch (error: any) {
      const message = error.response?.data?.message || '操作失败'
      ElMessage.error(message)
    } finally {
      submitting.value = false
    }
  })
}

async function toggleEnabled(server: UpstreamServer) {
  try {
    await api.put(`/api/upstreams/${server.id}`, { enabled: server.enabled })
    ElMessage.success(server.enabled ? '服务器已启用' : '服务器已禁用')
    fetchStatus()
  } catch (error: any) {
    server.enabled = !server.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}

async function confirmDelete(server: UpstreamServer) {
  try {
    await ElMessageBox.confirm(
      `确定要删除服务器 "${server.name}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await api.delete(`/api/upstreams/${server.id}`)
    ElMessage.success('服务器删除成功')
    fetchServers()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchServers()
  fetchStatus()
  // Refresh status every 30 seconds
  statusInterval = setInterval(fetchStatus, 30000)
})

onUnmounted(() => {
  if (statusInterval) {
    clearInterval(statusInterval)
  }
})
</script>

<style scoped>
.upstreams {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h1 {
  margin: 0;
}

.stats-cell {
  display: flex;
  flex-direction: column;
  font-size: 12px;
  color: #666;
}

.unit-label {
  margin-left: 10px;
  color: #999;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
