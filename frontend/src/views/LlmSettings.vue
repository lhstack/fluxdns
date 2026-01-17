<template>
  <div class="llm-settings-container">
    <div class="header-section">
      <div class="title-group">
        <h1 class="glow-text">AI 助手控制中心</h1>
        <p class="subtitle">配置大语言模型以驱动您的智能 DNS 管理专家</p>
      </div>
      <el-button type="primary" plain class="glass-btn" @click="fetchConfigs">
        <el-icon><Refresh /></el-icon>
        <span>刷新状态</span>
      </el-button>
    </div>

    <!-- 活跃配置展示区 (流光边框卡片) -->
    <div v-if="activeConfig" class="active-config-wrapper animated-border">
      <div class="active-config-content">
        <div class="active-info-main">
          <div class="active-provider">
            <div class="provider-icon-large" :style="{ background: getProviderColor(activeConfig.provider) }">
              {{ activeConfig.display_name.charAt(0) }}
            </div>
            <div class="provider-texts">
              <div class="status-indicator">
                <div class="pulse-dot"></div>
                <span>当前服务在线</span>
              </div>
              <h2 class="provider-name-text">{{ activeConfig.display_name }}</h2>
              <span class="model-tag">{{ activeConfig.model }}</span>
            </div>
          </div>
        </div>
        
        <div class="active-info-stats">
          <div class="stat-box">
            <span class="label">API Base URL</span>
            <span class="value">{{ activeConfig.api_base_url }}</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat-box">
            <span class="label">连接状态</span>
            <span class="value success-text">加密连接已建立</span>
          </div>
          <el-button 
            type="success" 
            class="neon-btn-compact" 
            @click="testConfig(activeConfig)" 
            :loading="testingId === activeConfig.id"
          >
            <el-icon><Connection /></el-icon>
            测试
          </el-button>
        </div>
      </div>
    </div>

    <div class="main-layout mt-4">
      <el-row :gutter="24">
        <!-- 厂商选择 (左侧) -->
        <el-col :xs="24" :lg="14">
          <div class="glass-card full-height">
            <div class="card-header">
              <el-icon><Grid /></el-icon>
              <span>选择 LLM 服务商</span>
            </div>
            
            <div class="providers-grid-container" v-loading="loadingProviders">
              <div
                v-for="provider in providers"
                :key="provider.name"
                class="provider-glass-item"
                :class="{ active: selectedProvider?.name === provider.name }"
                @click="selectProvider(provider)"
              >
                <div class="provider-main-info">
                  <div class="provider-badge-icon" :style="{ background: getProviderColor(provider.name) }">
                    {{ provider.display_name.charAt(0) }}
                  </div>
                  <div class="provider-text-content">
                    <span class="p-name">{{ provider.display_name }}</span>
                    <div class="p-status-row">
                      <el-tag v-if="getProviderConfig(provider.name)?.enabled" type="success" size="small" class="neon-tag-mini">Running</el-tag>
                      <span v-else-if="getProviderConfig(provider.name)" class="p-configured-text">已配置</span>
                      <span v-else class="p-models">{{ provider.models[0]?.split('-')[0] || 'Default' }}...</span>
                    </div>
                  </div>
                </div>
                
                <div class="card-overlay-actions" @click.stop>
                  <el-tooltip content="启用" placement="top" v-if="getProviderConfig(provider.name) && !getProviderConfig(provider.name)?.enabled">
                    <div class="mini-op-btn success" @click="enableConfig(getProviderConfig(provider.name)!.id)">
                      <el-icon><CircleCheck /></el-icon>
                    </div>
                  </el-tooltip>
                  <el-tooltip content="编辑" placement="top" v-if="getProviderConfig(provider.name)">
                    <div class="mini-op-btn primary" @click="editConfig(getProviderConfig(provider.name)!)">
                      <el-icon><Edit /></el-icon>
                    </div>
                  </el-tooltip>
                  <el-popconfirm title="删除?" @confirm="deleteConfig(getProviderConfig(provider.name)!.id)" v-if="getProviderConfig(provider.name)">
                    <template #reference>
                      <div class="mini-op-btn danger">
                        <el-icon><Delete /></el-icon>
                      </div>
                    </template>
                  </el-popconfirm>
                </div>

                <div class="provider-select-mark" v-if="selectedProvider?.name === provider.name && !getProviderConfig(provider.name)">
                  <el-icon><CircleCheckFilled /></el-icon>
                </div>
              </div>
            </div>
          </div>
        </el-col>

        <!-- 配置表单 (右侧) -->
        <el-col :xs="24" :lg="10">
          <div class="glass-card config-form-section">
            <div class="card-header">
              <el-icon><Setting /></el-icon>
              <span>{{ editingConfig ? '编辑连接参数' : '配置新连接' }}</span>
            </div>

            <el-form
              ref="formRef"
              :model="form"
              :rules="rules"
              label-position="top"
              class="premium-form"
              v-loading="saving"
            >
              <el-form-item label="厂商标识" prop="provider">
                <el-input v-model="form.provider" disabled class="glass-input" />
              </el-form-item>
              
              <el-form-item label="友好名称" prop="display_name">
                <el-input v-model="form.display_name" placeholder="例如: 我的开发助手" class="glass-input" />
              </el-form-item>

              <el-form-item label="API 端点 (Base URL)" prop="api_base_url">
                <el-input v-model="form.api_base_url" placeholder="https://..." class="glass-input" />
              </el-form-item>

              <el-form-item label="API 访问密钥" prop="api_key">
                <el-input
                  v-model="form.api_key"
                  type="password"
                  show-password
                  placeholder="sk-..."
                  class="glass-input"
                />
              </el-form-item>

              <el-form-item label="选择模型" prop="model">
                <el-select 
                  v-model="form.model" 
                  placeholder="选择预设或输入" 
                  filterable 
                  allow-create 
                  class="glass-select"
                >
                  <el-option
                    v-for="model in selectedProvider?.models || []"
                    :key="model"
                    :label="model"
                    :value="model"
                  />
                </el-select>
              </el-form-item>

              <div class="form-footer">
                <el-button @click="resetForm" class="glass-btn">取消</el-button>
                <el-button type="primary" class="glow-btn" @click="submitForm" :loading="saving">
                  {{ editingConfig ? '保存更新' : '创建配置' }}
                </el-button>
              </div>
            </el-form>
          </div>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  Refresh, Grid, Setting, Edit, Delete, Connection, CircleCheckFilled, CircleCheck
} from '@element-plus/icons-vue'
import api from '../api'

interface ProviderPreset {
  name: string
  display_name: string
  api_base_url: string
  models: string[]
}

interface LlmConfig {
  id: number
  provider: string
  display_name: string
  api_base_url: string
  api_key: string
  api_key_masked: string
  model: string
  enabled: boolean
}

const formRef = ref<FormInstance>()
const loadingProviders = ref(false)
const loadingConfigs = ref(false)
const saving = ref(false)
const testingId = ref<number | null>(null)
const enablingId = ref<number | null>(null)
const deletingId = ref<number | null>(null)

const providers = ref<ProviderPreset[]>([])
const configs = ref<LlmConfig[]>([])
const selectedProvider = ref<ProviderPreset | null>(null)
const editingConfig = ref<LlmConfig | null>(null)

const form = reactive({
  provider: '',
  display_name: '',
  api_base_url: '',
  api_key: '',
  model: ''
})

const rules: FormRules = {
  provider: [{ required: true, message: '请选择厂商', trigger: 'blur' }],
  api_base_url: [{ required: true, message: '请输入 API Base URL', trigger: 'blur' }],
  api_key: [{ required: true, message: '请输入 API Key', trigger: 'blur' }],
  model: [{ required: true, message: '请选择或输入模型', trigger: 'blur' }]
}

const activeConfig = computed(() => configs.value.find(c => c.enabled))

const providerColors: Record<string, string> = {
  openai: 'linear-gradient(135deg, #10a37f 0%, #1a7f5a 100%)',
  deepseek: 'linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)',
  qwen: 'linear-gradient(135deg, #ff6a00 0%, #ee0979 100%)',
  zhipu: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
  doubao: 'linear-gradient(135deg, #00c6fb 0%, #005bea 100%)',
  wenxin: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
  moonshot: 'linear-gradient(135deg, #1e3c72 0%, #2a5298 100%)',
  lingyiwanwu: 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)'
}

function getProviderColor(provider: string): string {
  return providerColors[provider.toLowerCase()] || 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
}

function getProviderConfig(providerName: string) {
  return configs.value.find(c => c.provider === providerName && c.enabled) 
    || configs.value.find(c => c.provider === providerName)
}

function selectProvider(provider: ProviderPreset) {
  const existingConfig = configs.value.find(c => c.provider === provider.name && c.enabled) 
    || configs.value.find(c => c.provider === provider.name)
  
  if (existingConfig) {
    editConfig(existingConfig)
    return
  }

  selectedProvider.value = provider
  form.provider = provider.name
  form.display_name = provider.display_name
  form.api_base_url = provider.api_base_url
  form.api_key = ''
  form.model = provider.models[0] || ''
  editingConfig.value = null
}

function editConfig(config: LlmConfig) {
  editingConfig.value = config
  form.provider = config.provider
  form.display_name = config.display_name
  form.api_base_url = config.api_base_url
  form.api_key = config.api_key
  form.model = config.model
  selectedProvider.value = providers.value.find(p => p.name === config.provider) || null
}

function resetForm() {
  formRef.value?.resetFields()
  editingConfig.value = null
  selectedProvider.value = null
}

async function fetchProviders() {
  loadingProviders.value = true
  try {
    const { data } = await api.get('/api/llm/providers')
    providers.value = data
  } catch (error) {
    ElMessage.error('获取厂商列表失败')
  } finally {
    loadingProviders.value = false
  }
}

async function fetchConfigs() {
  loadingConfigs.value = true
  try {
    const { data } = await api.get('/api/llm/config')
    configs.value = data
  } catch (error) {
    ElMessage.error('获取配置列表失败')
  } finally {
    loadingConfigs.value = false
  }
}

async function submitForm() {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    saving.value = true
    try {
      if (editingConfig.value) {
        await api.put(`/api/llm/config/${editingConfig.value.id}`, form)
        ElMessage.success('配置已更新')
      } else {
        await api.post('/api/llm/config', form)
        ElMessage.success('配置已添加')
      }
      resetForm()
      fetchConfigs()
    } catch (error) {
      ElMessage.error('保存失败')
    } finally {
      saving.value = false
    }
  })
}

async function enableConfig(id: number) {
  enablingId.value = id
  try {
    await api.post(`/api/llm/config/${id}/enable`)
    ElMessage.success('已开启智能分析助手')
    fetchConfigs()
  } catch (error) {
    ElMessage.error('开启失败')
  } finally {
    enablingId.value = null
  }
}

async function deleteConfig(id: number) {
  deletingId.value = id
  try {
    await api.delete(`/api/llm/config/${id}`)
    ElMessage.success('配置已移除')
    fetchConfigs()
  } catch (error) {
    ElMessage.error('移除失败')
  } finally {
    deletingId.value = null
  }
}

async function testConfig(config: LlmConfig) {
  testingId.value = config.id
  try {
    const apiKey = config.api_key || prompt('请输入 API Key 进行连接测试 (此操作不会保存):')
    if (!apiKey) {
      testingId.value = null
      return
    }
    const { data } = await api.post('/api/llm/config/test', {
      provider: config.provider,
      api_base_url: config.api_base_url,
      api_key: apiKey,
      model: config.model
    })
    if (data.success) {
      ElMessage({
        message: '连接成功! AI 服务已就绪。',
        type: 'success',
        duration: 3000
      })
    } else {
      ElMessage.error(data.message || '连接失败，请检查配置或网络')
    }
  } catch (error) {
    ElMessage.error('测试过程中发生未知网络错误')
  } finally {
    testingId.value = null
  }
}

onMounted(() => {
  fetchProviders()
  fetchConfigs()
})
</script>

<style scoped>
.llm-settings-container {
  padding: 30px;
  max-width: 1400px;
  margin: 0 auto;
  min-height: 100vh;
  background: radial-gradient(circle at top right, rgba(79, 172, 254, 0.05), transparent 400px),
              radial-gradient(circle at bottom left, rgba(0, 242, 254, 0.05), transparent 400px);
}

.header-section {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 30px;
}

.glow-text {
  margin: 0;
  font-size: 32px;
  font-weight: 800;
  background: linear-gradient(135deg, #fff, #4facfe);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  filter: drop-shadow(0 0 10px rgba(79, 172, 254, 0.3));
}

.subtitle {
  margin: 8px 0 0;
  color: rgba(255, 255, 255, 0.5);
  font-size: 15px;
}

/* 玻璃拟态卡片通用 */
.glass-card {
  background: rgba(30, 41, 59, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(16px);
  border-radius: 20px;
  padding: 24px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  transition: border-color 0.3s;
}

.glass-card:hover {
  border-color: rgba(255, 255, 255, 0.2);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
  font-size: 18px;
  font-weight: 700;
  color: rgba(255, 255, 255, 0.9);
}

.card-header.space-between {
  justify-content: space-between;
}

/* 活跃配置卡片 - 流光边框 */
.active-config-wrapper {
  position: relative;
  background: rgba(15, 23, 42, 0.6);
  border-radius: 24px;
  overflow: hidden;
  padding: 2px; /* 边框厚度 */
  margin-bottom: 30px;
}

.animated-border::before {
  content: '';
  position: absolute;
  top: -50%;
  left: -50%;
  width: 200%;
  height: 200%;
  background: conic-gradient(
    transparent, 
    #10b981, 
    #34d399,
    transparent 30%
  );
  animation: rotate 4s linear infinite;
  z-index: 0;
}

@keyframes rotate {
  100% { transform: rotate(360deg); }
}

/* 活跃配置卡片 - 极简流光 */
.active-config-wrapper {
  position: relative;
  background: rgba(15, 23, 42, 0.4);
  border-radius: 20px;
  overflow: hidden;
  padding: 1px;
  margin-bottom: 24px;
}

.active-config-content {
  position: relative;
  z-index: 1;
  background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
  border-radius: 19px;
  padding: 16px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 30px;
}

.active-info-main {
  display: flex;
  align-items: center;
}

.active-provider {
  display: flex;
  align-items: center;
  gap: 20px;
}

.provider-icon-large {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  color: white;
  font-weight: 800;
  box-shadow: 0 0 15px rgba(0, 0, 0, 0.3);
}

.provider-texts {
  display: flex;
  flex-direction: column;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #10b981;
  font-weight: 700;
  text-transform: uppercase;
  margin-bottom: 2px;
}

.pulse-dot {
  width: 6px;
  height: 6px;
  background: #10b981;
  border-radius: 50%;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% { transform: scale(0.9); opacity: 0.6; }
  50% { transform: scale(1.1); opacity: 1; }
  100% { transform: scale(0.9); opacity: 0.6; }
}

.provider-name-text {
  margin: 0;
  font-size: 20px;
  color: white;
  line-height: 1.2;
}

.model-tag {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.3);
  margin-top: 2px;
}

.active-info-stats {
  display: flex;
  align-items: center;
  gap: 40px;
  background: rgba(0, 0, 0, 0.2);
  padding: 10px 24px;
  border-radius: 12px;
}

.stat-box {
  display: flex;
  flex-direction: column;
}

.stat-box .label {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.3);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stat-box .value {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.7);
  font-family: 'JetBrains Mono', monospace;
}

.stat-divider {
  width: 1px;
  height: 24px;
  background: rgba(255, 255, 255, 0.05);
}

.neon-btn-compact {
  height: 30px !important;
  background: rgba(16, 185, 129, 0.1) !important;
  border: 1px solid rgba(16, 185, 129, 0.3) !important;
  color: #10b981 !important;
  font-size: 12px !important;
  border-radius: 6px !important;
}

/* 厂商选择网格 - 优化协调性 */
.providers-grid-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.provider-glass-item {
  position: relative;
  display: flex;
  align-items: center;
  padding: 16px 20px;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.23, 1, 0.32, 1);
  min-height: 80px;
}

.provider-main-info {
  display: flex;
  align-items: center;
  gap: 16px;
  flex: 1;
  min-width: 0;
}

.provider-badge-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 18px;
  font-weight: 700;
  flex-shrink: 0;
}

.provider-text-content {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.p-name {
  font-weight: 600;
  color: white;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.p-status-row {
  margin-top: 4px;
  height: 20px;
  display: flex;
  align-items: center;
}

.neon-tag-mini {
  background: rgba(16, 185, 129, 0.15) !important;
  border-color: rgba(16, 185, 129, 0.3) !important;
  color: #10b981 !important;
  font-size: 9px !important;
  padding: 0 4px !important;
  line-height: 16px !important;
  height: 16px !important;
}

.p-configured-text {
  font-size: 11px;
  color: #4facfe;
}

.p-models {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.3);
}

.card-overlay-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
  margin-left: 10px;
}

.provider-glass-item:hover .card-overlay-actions {
  opacity: 1;
}

.mini-op-btn {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.05);
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
}

.mini-op-btn:hover { background: rgba(255, 255, 255, 0.1); color: white; }
.mini-op-btn.success:hover { background: #10b981; }
.mini-op-btn.primary:hover { background: #4facfe; }
.mini-op-btn.danger:hover { background: #f56c6c; }

.provider-select-mark {
  color: #4facfe;
  font-size: 16px;
  margin-left: 10px;
}

/* 右侧配置表单优化 */
.config-form-section {
  position: sticky;
  top: 30px;
}

.premium-form :deep(.el-form-item) {
  margin-bottom: 20px;
}

.form-footer {
  margin-top: 30px;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

@media (max-width: 1200px) {
  .active-config-content {
    flex-direction: column;
    align-items: flex-start;
  }
  .active-info-stats {
    width: 100%;
    justify-content: space-between;
  }
}

@media (max-width: 600px) {
  .providers-grid-container {
    grid-template-columns: 1fr;
  }
}
</style>
