<template>
  <div class="listeners">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-left">
        <h1>服务监听配置</h1>
        <p class="subtitle">配置 DNS 服务器监听的协议和端口，支持 UDP、DoT、DoH、DoQ、DoH3 等协议</p>
      </div>
      <el-button type="primary" size="large" @click="fetchListeners">
        <el-icon><Refresh /></el-icon>
        刷新
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            <el-icon><Connection /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ listeners.length }}</span>
            <span class="stat-label">监听协议</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);">
            <el-icon><CircleCheck /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ enabledCount }}</span>
            <span class="stat-label">已启用</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #f5576c 0%, #f093fb 100%);">
            <el-icon><Lock /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ tlsConfiguredCount }}</span>
            <span class="stat-label">TLS 已配置</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ needsTlsCount }}</span>
            <span class="stat-label">待配置 TLS</span>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 监听器卡片 -->
    <el-row :gutter="20" v-loading="loading">
      <el-col :xs="24" :md="12" v-for="listener in listeners" :key="listener.protocol">
        <el-card class="listener-card" :class="{ 'is-enabled': listener.enabled }" shadow="never">
          <template #header>
            <div class="card-header">
              <div class="protocol-info">
                <div class="protocol-badge" :style="{ background: getProtocolGradient(listener.protocol) }">
                  {{ listener.protocol.toUpperCase() }}
                </div>
                <div class="protocol-meta">
                  <span class="protocol-name">{{ getProtocolName(listener.protocol) }}</span>
                  <span class="protocol-desc">{{ listener.description }}</span>
                </div>
              </div>
              <el-switch
                v-model="listener.enabled"
                @change="toggleListener(listener)"
                :disabled="saving[listener.protocol]"
                inline-prompt
                active-text="启"
                inactive-text="停"
                size="large"
              />
            </div>
          </template>

          <el-form label-position="top" size="default">
            <el-row :gutter="16">
              <el-col :span="12">
                <el-form-item label="绑定地址">
                  <el-input
                    v-model="listener.bind_address"
                    placeholder="0.0.0.0"
                    :disabled="!listener.enabled"
                  >
                    <template #prefix>
                      <el-icon><Location /></el-icon>
                    </template>
                  </el-input>
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="端口">
                  <el-input-number
                    v-model="listener.port"
                    :min="1"
                    :max="65535"
                    :disabled="!listener.enabled"
                    style="width: 100%"
                  />
                </el-form-item>
              </el-col>
            </el-row>

            <template v-if="listener.requires_tls">
              <div class="tls-section">
                <div class="tls-header">
                  <el-icon><Lock /></el-icon>
                  <span>TLS 证书配置</span>
                </div>
                
                <div class="tls-status">
                  <div class="status-item">
                    <span class="status-label">证书</span>
                    <div class="status-actions">
                      <el-button 
                        type="primary" 
                        size="small" 
                        @click="openCertDialog(listener, 'cert')"
                        :disabled="!listener.enabled"
                      >
                        {{ listener.has_tls_cert ? '更新证书' : '配置证书' }}
                      </el-button>
                      <el-button 
                        v-if="listener.has_tls_cert"
                        type="danger" 
                        size="small" 
                        @click="clearCert(listener, 'cert')"
                        :disabled="!listener.enabled"
                      >
                        清除
                      </el-button>
                      <el-tag :type="listener.has_tls_cert ? 'success' : 'danger'" size="small" effect="plain">
                        {{ listener.has_tls_cert ? '已配置' : '未配置' }}
                      </el-tag>
                    </div>
                  </div>
                  <div class="status-item">
                    <span class="status-label">私钥</span>
                    <div class="status-actions">
                      <el-button 
                        type="primary" 
                        size="small" 
                        @click="openCertDialog(listener, 'key')"
                        :disabled="!listener.enabled"
                      >
                        {{ listener.has_tls_key ? '更新私钥' : '配置私钥' }}
                      </el-button>
                      <el-button 
                        v-if="listener.has_tls_key"
                        type="danger" 
                        size="small" 
                        @click="clearCert(listener, 'key')"
                        :disabled="!listener.enabled"
                      >
                        清除
                      </el-button>
                      <el-tag :type="listener.has_tls_key ? 'success' : 'danger'" size="small" effect="plain">
                        {{ listener.has_tls_key ? '已配置' : '未配置' }}
                      </el-tag>
                    </div>
                  </div>
                </div>

                <el-alert
                  v-if="listener.enabled && (!listener.has_tls_cert || !listener.has_tls_key)"
                  type="warning"
                  :closable="false"
                  show-icon
                  class="tls-warning"
                >
                  需要配置 TLS 证书和私钥才能启动此服务
                </el-alert>
              </div>
            </template>

            <div class="card-footer">
              <el-button
                type="primary"
                @click="saveListener(listener)"
                :loading="saving[listener.protocol]"
                :disabled="!listener.enabled"
              >
                <el-icon><Check /></el-icon>
                保存配置
              </el-button>
            </div>
          </el-form>
        </el-card>
      </el-col>
    </el-row>

    <!-- 提示信息 -->
    <el-alert
      type="info"
      show-icon
      :closable="false"
      class="notice-alert"
    >
      <template #title>
        <span class="alert-title">配置提示</span>
      </template>
      修改监听配置后需要重启服务才能生效。TLS 协议（DoT、DoH、DoQ、DoH3）需要配置有效的证书和私钥。
    </el-alert>

    <!-- 证书配置对话框 -->
    <el-dialog
      v-model="certDialogVisible"
      :title="certDialogTitle"
      width="600px"
      :close-on-click-modal="false"
      class="cert-dialog"
    >
      <el-tabs v-model="certInputMode">
        <el-tab-pane label="粘贴内容" name="paste">
          <el-input
            v-model="certContent"
            type="textarea"
            :rows="12"
            :placeholder="certPlaceholder"
            class="cert-textarea"
          />
        </el-tab-pane>
        <el-tab-pane label="上传文件" name="upload">
          <el-upload
            class="cert-upload"
            drag
            :auto-upload="false"
            :show-file-list="false"
            @change="handleFileChange"
            accept=".pem,.crt,.key,.cer"
          >
            <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
            <div class="el-upload__text">
              拖拽文件到此处，或 <em>点击上传</em>
            </div>
            <template #tip>
              <div class="el-upload__tip">
                支持 .pem, .crt, .key, .cer 格式
              </div>
            </template>
          </el-upload>
          <el-input
            v-if="certContent"
            v-model="certContent"
            type="textarea"
            :rows="8"
            readonly
            class="uploaded-content"
          />
        </el-tab-pane>
      </el-tabs>
      <template #footer>
        <el-button @click="certDialogVisible = false" size="large">取消</el-button>
        <el-button type="primary" @click="saveCert" :loading="savingCert" size="large">
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { 
  Refresh, Connection, CircleCheck, Lock, Warning, 
  Location, Check, UploadFilled 
} from '@element-plus/icons-vue'
import api from '../api'

interface Listener {
  protocol: string
  enabled: boolean
  bind_address: string
  port: number
  has_tls_cert: boolean
  has_tls_key: boolean
  requires_tls: boolean
  description: string
}

const listeners = ref<Listener[]>([])
const loading = ref(false)
const saving = reactive<Record<string, boolean>>({})

// 统计数据
const enabledCount = computed(() => listeners.value.filter(l => l.enabled).length)
const tlsConfiguredCount = computed(() => 
  listeners.value.filter(l => l.requires_tls && l.has_tls_cert && l.has_tls_key).length
)
const needsTlsCount = computed(() => 
  listeners.value.filter(l => l.enabled && l.requires_tls && (!l.has_tls_cert || !l.has_tls_key)).length
)

// 证书对话框
const certDialogVisible = ref(false)
const certInputMode = ref('paste')
const certContent = ref('')
const certType = ref<'cert' | 'key'>('cert')
const currentListener = ref<Listener | null>(null)
const savingCert = ref(false)

const certDialogTitle = computed(() => {
  if (!currentListener.value) return ''
  const protocol = currentListener.value.protocol.toUpperCase()
  return certType.value === 'cert' ? `${protocol} - 配置 TLS 证书` : `${protocol} - 配置 TLS 私钥`
})

const certPlaceholder = computed(() => {
  return certType.value === 'cert' 
    ? '-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----'
    : '-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----'
})

function getProtocolGradient(protocol: string): string {
  const gradients: Record<string, string> = {
    udp: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
    dot: 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)',
    doh: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
    doq: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)',
    doh3: 'linear-gradient(135deg, #fa709a 0%, #fee140 100%)'
  }
  return gradients[protocol] ?? gradients.udp ?? ''
}

function getProtocolName(protocol: string): string {
  const names: Record<string, string> = {
    udp: 'DNS over UDP',
    dot: 'DNS over TLS',
    doh: 'DNS over HTTPS',
    doq: 'DNS over QUIC',
    doh3: 'DNS over HTTP/3'
  }
  return names[protocol] || protocol.toUpperCase()
}

async function fetchListeners() {
  loading.value = true
  try {
    const response = await api.get('/api/listeners')
    listeners.value = response.data.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取监听配置失败')
  } finally {
    loading.value = false
  }
}

async function toggleListener(listener: Listener) {
  saving[listener.protocol] = true
  try {
    await api.put(`/api/listeners/${listener.protocol}`, {
      enabled: listener.enabled
    })
    if (listener.enabled && listener.requires_tls && (!listener.has_tls_cert || !listener.has_tls_key)) {
      ElMessage.warning(`${listener.protocol.toUpperCase()} 已启用，请配置 TLS 证书`)
    } else {
      ElMessage.success(listener.enabled ? `${listener.protocol.toUpperCase()} 已启用` : `${listener.protocol.toUpperCase()} 已禁用`)
    }
  } catch (error: any) {
    listener.enabled = !listener.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  } finally {
    saving[listener.protocol] = false
  }
}

async function saveListener(listener: Listener) {
  saving[listener.protocol] = true
  try {
    const response = await api.put(`/api/listeners/${listener.protocol}`, {
      enabled: listener.enabled,
      bind_address: listener.bind_address,
      port: listener.port
    })
    Object.assign(listener, response.data)
    ElMessage.success(`${listener.protocol.toUpperCase()} 配置已保存`)
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存失败')
  } finally {
    saving[listener.protocol] = false
  }
}

function openCertDialog(listener: Listener, type: 'cert' | 'key') {
  currentListener.value = listener
  certType.value = type
  certContent.value = ''
  certInputMode.value = 'paste'
  certDialogVisible.value = true
}

function handleFileChange(file: any) {
  const reader = new FileReader()
  reader.onload = (e) => {
    certContent.value = e.target?.result as string
  }
  reader.readAsText(file.raw)
}

async function saveCert() {
  if (!currentListener.value || !certContent.value.trim()) {
    ElMessage.warning('请输入或上传证书内容')
    return
  }

  savingCert.value = true
  try {
    const payload: any = {}
    if (certType.value === 'cert') {
      payload.tls_cert = certContent.value
    } else {
      payload.tls_key = certContent.value
    }

    const response = await api.put(`/api/listeners/${currentListener.value.protocol}`, payload)
    
    const idx = listeners.value.findIndex(l => l.protocol === currentListener.value?.protocol)
    if (idx !== -1 && listeners.value[idx]) {
      Object.assign(listeners.value[idx], response.data)
    }

    ElMessage.success(certType.value === 'cert' ? '证书已保存' : '私钥已保存')
    certDialogVisible.value = false
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存失败')
  } finally {
    savingCert.value = false
  }
}

async function clearCert(listener: Listener, type: 'cert' | 'key') {
  try {
    await ElMessageBox.confirm(
      `确定要清除 ${listener.protocol.toUpperCase()} 的${type === 'cert' ? '证书' : '私钥'}吗？`,
      '确认',
      { type: 'warning' }
    )

    saving[listener.protocol] = true
    const payload: any = {}
    if (type === 'cert') {
      payload.tls_cert = ''
    } else {
      payload.tls_key = ''
    }

    const response = await api.put(`/api/listeners/${listener.protocol}`, payload)
    Object.assign(listener, response.data)
    ElMessage.success('已清除')
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '操作失败')
    }
  } finally {
    saving[listener.protocol] = false
  }
}

onMounted(() => {
  fetchListeners()
})
</script>

<style scoped>
.listeners {
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

/* 监听器卡片 */
.listener-card {
  border-radius: 12px;
  border: 2px solid transparent;
  margin-bottom: 20px;
  transition: all 0.3s;
}

.listener-card.is-enabled {
  border-color: #67c23a;
}

.listener-card :deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #f0f0f0;
}

.listener-card :deep(.el-card__body) {
  padding: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.protocol-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.protocol-badge {
  padding: 8px 16px;
  border-radius: 8px;
  color: #fff;
  font-weight: 600;
  font-size: 14px;
}

.protocol-meta {
  display: flex;
  flex-direction: column;
}

.protocol-name {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}

.protocol-desc {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

/* TLS 配置区域 */
.tls-section {
  background: #f8f9fa;
  border-radius: 8px;
  padding: 16px;
  margin-top: 16px;
}

.tls-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
}

.tls-header .el-icon {
  color: #667eea;
}

.tls-status {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-label {
  font-size: 13px;
  color: #606266;
  width: 40px;
  flex-shrink: 0;
}

.status-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tls-warning {
  margin-top: 12px;
}

.card-footer {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

/* 提示信息 */
.notice-alert {
  margin-top: 24px;
  border-radius: 8px;
}

.alert-title {
  font-weight: 600;
}

/* 证书对话框 */
.cert-dialog :deep(.el-dialog__header) {
  border-bottom: 1px solid #f0f0f0;
  padding: 20px 24px;
}

.cert-dialog :deep(.el-dialog__body) {
  padding: 24px;
}

.cert-dialog :deep(.el-dialog__footer) {
  border-top: 1px solid #f0f0f0;
  padding: 16px 24px;
}

.cert-textarea :deep(.el-textarea__inner) {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
}

.cert-upload {
  width: 100%;
}

.uploaded-content {
  margin-top: 16px;
}

.uploaded-content :deep(.el-textarea__inner) {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 12px;
  background: #f8f9fa;
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
  
  .protocol-info {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
