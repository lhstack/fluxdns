<template>
  <div class="login-container">
    <div class="login-box">
      <!-- Logo 和标题 -->
      <div class="login-header">
        <img src="/logo.svg" alt="FluxDNS" class="logo" />
        <h1 class="title">FluxDNS</h1>
        <p class="subtitle">现代化 DNS 代理服务</p>
      </div>

      <!-- 登录表单 -->
      <el-card class="login-card" shadow="hover">
        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          label-position="top"
          @submit.prevent="handleLogin"
        >
          <el-form-item label="用户名" prop="username">
            <el-input 
              v-model="form.username" 
              placeholder="请输入用户名"
              size="large"
              prefix-icon="User"
            />
          </el-form-item>
          <el-form-item label="密码" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              placeholder="请输入密码"
              size="large"
              prefix-icon="Lock"
              show-password
            />
          </el-form-item>
          <el-form-item>
            <el-button 
              type="primary" 
              native-type="submit" 
              :loading="loading" 
              size="large"
              class="login-btn"
            >
              登 录
            </el-button>
          </el-form-item>
        </el-form>
      </el-card>

      <!-- 底部信息 -->
      <div class="login-footer">
        <span>支持 UDP / DoT / DoH / DoQ / DoH3 协议</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const formRef = ref<FormInstance>()
const loading = ref(false)

const form = reactive({
  username: '',
  password: ''
})

const rules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }]
}

async function handleLogin() {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    loading.value = true
    try {
      await authStore.login(form.username, form.password)
      ElMessage.success('登录成功')
      router.push('/')
    } catch (error: unknown) {
      const err = error as { response?: { data?: { message?: string } } }
      ElMessage.error(err.response?.data?.message || '登录失败')
    } finally {
      loading.value = false
    }
  })
}
</script>

<style scoped>
.login-container {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 20px;
}

.login-box {
  width: 100%;
  max-width: 420px;
}

.login-header {
  text-align: center;
  margin-bottom: 30px;
}

.logo {
  width: 80px;
  height: 80px;
  margin-bottom: 16px;
  filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.1));
}

.title {
  margin: 0 0 8px 0;
  font-size: 32px;
  font-weight: 600;
  color: #fff;
  letter-spacing: 2px;
}

.subtitle {
  margin: 0;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.8);
}

.login-card {
  border-radius: 12px;
  border: none;
}

.login-card :deep(.el-card__body) {
  padding: 32px;
}

.login-card :deep(.el-form-item__label) {
  font-weight: 500;
  color: #606266;
}

.login-card :deep(.el-input__wrapper) {
  border-radius: 8px;
}

.login-btn {
  width: 100%;
  margin-top: 8px;
  border-radius: 8px;
  font-size: 16px;
  letter-spacing: 4px;
}

.login-footer {
  text-align: center;
  margin-top: 24px;
  color: rgba(255, 255, 255, 0.7);
  font-size: 12px;
}
</style>
