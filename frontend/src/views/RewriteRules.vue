<template>
  <div class="rewrite-rules">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-left">
        <h1>重写规则管理</h1>
        <p class="subtitle">配置 DNS 查询重写规则，支持精确匹配、通配符和正则表达式</p>
      </div>
      <el-button type="primary" size="large" @click="openCreateDialog">
        <el-icon><Plus /></el-icon>
        添加规则
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            <el-icon><Edit /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ rules.length }}</span>
            <span class="stat-label">总规则数</span>
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
            <el-icon><CloseBold /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ blockCount }}</span>
            <span class="stat-label">阻止规则</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);">
            <el-icon><Switch /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ mapCount }}</span>
            <span class="stat-label">映射规则</span>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 规则表格 -->
    <el-card class="table-card" shadow="never">
      <el-table :data="rules" v-loading="loading" stripe class="custom-table">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="pattern" label="匹配模式" min-width="200">
          <template #default="{ row }">
            <span class="pattern-text">{{ row.pattern }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="match_type" label="匹配类型" width="110">
          <template #default="{ row }">
            <el-tag :type="getMatchTypeTag(row.match_type)" effect="plain">
              {{ getMatchTypeLabel(row.match_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="action_type" label="动作" width="110">
          <template #default="{ row }">
            <el-tag :type="getActionTypeTag(row.action_type)" effect="dark">
              {{ getActionTypeLabel(row.action_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="action_value" label="动作值" min-width="150">
          <template #default="{ row }">
            <span class="action-value">{{ row.action_value || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="priority" label="优先级" width="90" />
        <el-table-column prop="enabled" label="状态" width="90">
          <template #default="{ row }">
            <el-switch
              v-model="row.enabled"
              @change="toggleEnabled(row)"
              inline-prompt
              active-text="启"
              inactive-text="停"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="140" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link @click="openEditDialog(row)">
              <el-icon><Edit /></el-icon> 编辑
            </el-button>
            <el-button type="danger" link @click="confirmDelete(row)">
              <el-icon><Delete /></el-icon> 删除
            </el-button>
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="暂无重写规则" />
        </template>
      </el-table>
    </el-card>

    <!-- 创建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑规则' : '添加规则'"
      width="560px"
      class="custom-dialog"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-position="top"
      >
        <el-form-item label="匹配模式" prop="pattern">
          <el-input
            v-model="formData.pattern"
            :placeholder="getPatternPlaceholder(formData.match_type)"
            size="large"
          />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="匹配类型" prop="match_type">
              <el-select v-model="formData.match_type" placeholder="选择匹配类型" size="large" style="width: 100%">
                <el-option label="精确匹配" value="exact" />
                <el-option label="通配符" value="wildcard" />
                <el-option label="正则表达式" value="regex" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="动作类型" prop="action_type">
              <el-select v-model="formData.action_type" placeholder="选择动作类型" size="large" style="width: 100%">
                <el-option label="映射到 IP" value="map_ip" />
                <el-option label="映射到域名" value="map_domain" />
                <el-option label="阻止" value="block" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item
          v-if="formData.action_type !== 'block'"
          label="动作值"
          prop="action_value"
        >
          <el-input
            v-model="formData.action_value"
            :placeholder="getActionValuePlaceholder(formData.action_type)"
            size="large"
          />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="优先级" prop="priority">
              <el-input-number v-model="formData.priority" :min="0" size="large" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="enabled">
              <el-switch v-model="formData.enabled" active-text="启用" inactive-text="禁用" size="large" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="formData.description"
            type="textarea"
            :rows="2"
            placeholder="规则描述（可选）"
            size="large"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false" size="large">取消</el-button>
        <el-button type="primary" @click="submitForm" :loading="submitting" size="large">
          {{ isEditing ? '保存修改' : '创建规则' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Edit, Delete, CircleCheck, CloseBold, Switch } from '@element-plus/icons-vue'
import api from '../api'

interface RewriteRule {
  id: number
  pattern: string
  match_type: string
  action_type: string
  action_value: string | null
  priority: number
  enabled: boolean
  description: string | null
  created_at: string
  updated_at: string
}

const rules = ref<RewriteRule[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const isEditing = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editingId = ref<number | null>(null)

const enabledCount = computed(() => rules.value.filter(r => r.enabled).length)
const blockCount = computed(() => rules.value.filter(r => r.action_type === 'block').length)
const mapCount = computed(() => rules.value.filter(r => r.action_type !== 'block').length)

const formData = reactive({
  pattern: '',
  match_type: 'exact',
  action_type: 'block',
  action_value: '',
  priority: 0,
  description: '',
  enabled: true
})

const formRules: FormRules = {
  pattern: [
    { required: true, message: '请输入匹配模式', trigger: 'blur' },
    { max: 255, message: '匹配模式长度不能超过255个字符', trigger: 'blur' }
  ],
  match_type: [
    { required: true, message: '请选择匹配类型', trigger: 'change' }
  ],
  action_type: [
    { required: true, message: '请选择动作类型', trigger: 'change' }
  ]
}

function getMatchTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    exact: '精确匹配',
    wildcard: '通配符',
    regex: '正则'
  }
  return labels[type] || type
}

function getMatchTypeTag(type: string): string {
  const tags: Record<string, string> = {
    exact: '',
    wildcard: 'warning',
    regex: 'danger'
  }
  return tags[type] || ''
}

function getActionTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    map_ip: '映射 IP',
    map_domain: '映射域名',
    block: '阻止'
  }
  return labels[type] || type
}

function getActionTypeTag(type: string): string {
  const tags: Record<string, string> = {
    map_ip: 'success',
    map_domain: 'warning',
    block: 'danger'
  }
  return tags[type] || ''
}

function getPatternPlaceholder(matchType: string): string {
  const placeholders: Record<string, string> = {
    exact: 'ads.example.com',
    wildcard: '*.ads.com',
    regex: '^ads?\\.'
  }
  return placeholders[matchType] || ''
}

function getActionValuePlaceholder(actionType: string): string {
  const placeholders: Record<string, string> = {
    map_ip: '192.168.1.1 或 ::1',
    map_domain: 'target.example.com'
  }
  return placeholders[actionType] || ''
}

async function fetchRules() {
  loading.value = true
  try {
    const response = await api.get('/api/rewrite')
    rules.value = response.data.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取规则失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  formData.pattern = ''
  formData.match_type = 'exact'
  formData.action_type = 'block'
  formData.action_value = ''
  formData.priority = 0
  formData.description = ''
  formData.enabled = true
  editingId.value = null
}

function openCreateDialog() {
  isEditing.value = false
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(rule: RewriteRule) {
  isEditing.value = true
  editingId.value = rule.id
  formData.pattern = rule.pattern
  formData.match_type = rule.match_type
  formData.action_type = rule.action_type
  formData.action_value = rule.action_value || ''
  formData.priority = rule.priority
  formData.description = rule.description || ''
  formData.enabled = rule.enabled
  dialogVisible.value = true
}

async function submitForm() {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitting.value = true
    try {
      const payload = {
        ...formData,
        action_value: formData.action_type === 'block' ? null : formData.action_value || null,
        description: formData.description || null
      }
      
      if (isEditing.value && editingId.value) {
        await api.put(`/api/rewrite/${editingId.value}`, payload)
        ElMessage.success('规则更新成功')
      } else {
        await api.post('/api/rewrite', payload)
        ElMessage.success('规则创建成功')
      }
      dialogVisible.value = false
      fetchRules()
    } catch (error: any) {
      const message = error.response?.data?.message || '操作失败'
      ElMessage.error(message)
    } finally {
      submitting.value = false
    }
  })
}

async function toggleEnabled(rule: RewriteRule) {
  try {
    await api.put(`/api/rewrite/${rule.id}`, { enabled: rule.enabled })
    ElMessage.success(rule.enabled ? '规则已启用' : '规则已禁用')
  } catch (error: any) {
    rule.enabled = !rule.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}

async function confirmDelete(rule: RewriteRule) {
  try {
    await ElMessageBox.confirm(
      `确定要删除规则 "${rule.pattern}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await api.delete(`/api/rewrite/${rule.id}`)
    ElMessage.success('规则删除成功')
    fetchRules()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchRules()
})
</script>

<style scoped>
.rewrite-rules {
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

/* 表格卡片 */
.table-card {
  border-radius: 12px;
  border: none;
}

.table-card :deep(.el-card__body) {
  padding: 0;
}

.custom-table :deep(.el-table__header th) {
  background: #f8f9fa;
  color: #606266;
  font-weight: 600;
}

.pattern-text {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  color: #303133;
}

.action-value {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  color: #606266;
}

/* 对话框 */
.custom-dialog :deep(.el-dialog__header) {
  border-bottom: 1px solid #f0f0f0;
  padding: 20px 24px;
}

.custom-dialog :deep(.el-dialog__body) {
  padding: 24px;
}

.custom-dialog :deep(.el-dialog__footer) {
  border-top: 1px solid #f0f0f0;
  padding: 16px 24px;
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
}
</style>
