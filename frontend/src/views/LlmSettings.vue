<template>
  <div class="llm-settings-container">
    <div class="header-section">
      <div class="title-group">
        <h1 class="page-title">AI 助手控制中心</h1>
        <p class="subtitle">配置大语言模型以驱动您的智能 DNS 管理专家</p>
      </div>
      <el-button type="primary" plain class="action-btn" @click="fetchConfigs">
        <el-icon><Refresh /></el-icon>
        <span>刷新状态</span>
      </el-button>
    </div>

    <div v-if="activeConfig" class="active-config-banner">
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
            size="small"
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
      <el-row :gutter="24" class="equal-height-row">
        <!-- 厂商选择 (左侧) -->
        <el-col :xs="24" :lg="16">
          <div class="settings-card full-height-card">
            <div class="card-header">
              <el-icon><Grid /></el-icon>
              <span>选择 LLM 服务商</span>
            </div>
            
            <div class="providers-grid-container" v-loading="loadingProviders">
              <div
                v-for="provider in providers"
                :key="provider.name"
                class="provider-card"
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
                      <el-tag v-if="getProviderConfig(provider.name)?.enabled" type="success" size="small">在线</el-tag>
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
        <el-col :xs="24" :lg="8">
          <div class="settings-card config-form-section full-height-card">
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
                <el-input v-model="form.provider" disabled />
              </el-form-item>
              
              <el-form-item label="友好名称" prop="display_name">
                <el-input v-model="form.display_name" placeholder="例如: 我的开发助手" />
              </el-form-item>

              <el-form-item label="API 端点 (Base URL)" prop="api_base_url">
                <el-input v-model="form.api_base_url" placeholder="https://..." />
              </el-form-item>

              <el-form-item label="API 访问密钥" prop="api_key">
                <el-input
                  v-model="form.api_key"
                  type="password"
                  show-password
                  placeholder="sk-..."
                />
              </el-form-item>

              <el-form-item label="选择模型" prop="model">
                <el-select 
                  v-model="form.model" 
                  placeholder="选择预设或输入" 
                  filterable 
                  allow-create 
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
                <el-button @click="resetForm" class="action-btn">取消</el-button>
                <el-button type="primary" @click="submitForm" :loading="saving">
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
        resetForm()
      }
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
}

.header-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.page-title {
  margin: 0;
  font-size: 32px;
  font-weight: 800;
  color: #303133;
}

.subtitle {
  margin: 8px 0 0;
  color: #606266;
  font-size: 15px;
}

/* 标准白色卡片 */
.settings-card {
  background: #fff;
  border: 1px solid #e4e7ed;
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  transition: all 0.3s;
  display: flex;
  flex-direction: column;
}

.full-height-card {
  height: 100%;
}

.equal-height-row {
  display: flex;
  flex-wrap: wrap;
}

.equal-height-row > .el-col {
  display: flex;
  flex-direction: column;
}

.settings-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
  font-size: 18px;
  font-weight: 700;
  color: #303133;
}

/* 活跃配置横幅 */
.active-config-banner {
  background: #fff;
  border: 1px solid #e4e7ed;
  border-radius: 12px;
  overflow: hidden;
  margin-bottom: 24px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.active-config-content {
  padding: 18px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 40px;
}

.active-provider {
  display: flex;
  align-items: center;
  gap: 16px;
}

.provider-icon-large {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  color: white;
  font-weight: 800;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #67c23a;
  font-weight: 600;
  margin-bottom: 4px;
}

.pulse-dot {
  width: 8px;
  height: 8px;
  background: #67c23a;
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
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.model-tag {
  font-size: 13px;
  color: #909399;
  margin-top: 2px;
}

.active-info-stats {
  display: flex;
  align-items: center;
  gap: 32px;
}

.stat-box .label {
  font-size: 11px;
  color: #909399;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.stat-box .value {
  font-size: 14px;
  color: #606266;
  font-family: 'JetBrains Mono', monospace;
}

.stat-divider {
  width: 1px;
  height: 32px;
  background: #ebeef5;
}

/* 厂商选择网格 */
.providers-grid-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
  flex: 1;
}

.provider-card {
  position: relative;
  display: flex;
  align-items: center;
  padding: 20px;
  border-radius: 12px;
  background: #f8f9fa;
  border: 1px solid #ebeef5;
  cursor: pointer;
  transition: all 0.3s;
  overflow: hidden;
}

.provider-card:hover {
  background: #f0f2f5;
  border-color: #dcdfe6;
  transform: translateY(-2px);
}

.provider-card.active {
  background: #f0f7ff;
  border-color: #667eea;
  box-shadow: 0 0 0 1px #667eea;
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
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
}

.p-name {
  font-weight: 600;
  color: #303133;
}

.p-configured-text {
  font-size: 11px;
  color: #667eea;
}

.p-models {
  font-size: 11px;
  color: #909399;
}

.card-overlay-actions {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  gap: 6px;
  opacity: 0;
  transform: translateY(-5px);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.provider-card:hover .card-overlay-actions {
  opacity: 1;
  transform: translateY(0);
}

.mini-op-btn {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #fff;
  border: 1px solid #dcdfe6;
  color: #606266;
  font-size: 12px;
  transition: all 0.2s;
}

.mini-op-btn:hover { border-color: transparent; color: white; }
.mini-op-btn.success:hover { background: #67c23a; }
.mini-op-btn.primary:hover { background: #667eea; }
.mini-op-btn.danger:hover { background: #f56c6c; }

.provider-select-mark {
  position: absolute;
  bottom: 8px;
  right: 8px;
  color: #667eea;
  font-size: 18px;
}

/* 表单样式调整 */
.premium-form :deep(.el-form-item__label) {
  color: #606266;
  font-weight: 600;
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
