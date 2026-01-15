<template>
  <el-container class="layout-container">
    <el-aside width="240px" class="sidebar">
      <!-- Logo -->
      <div class="logo">
        <img src="/logo.svg" alt="FluxDNS" class="logo-icon" />
        <span class="logo-text">FluxDNS</span>
      </div>

      <!-- 导航菜单 -->
      <el-menu
        :default-active="activeMenu"
        router
        class="sidebar-menu"
      >
        <el-menu-item index="/">
          <el-icon><Odometer /></el-icon>
          <span>仪表盘</span>
        </el-menu-item>
        <el-menu-item index="/records">
          <el-icon><Document /></el-icon>
          <span>DNS 记录</span>
        </el-menu-item>
        <el-menu-item index="/rewrite">
          <el-icon><Edit /></el-icon>
          <span>重写规则</span>
        </el-menu-item>
        <el-menu-item index="/upstreams">
          <el-icon><Connection /></el-icon>
          <span>上游服务器</span>
        </el-menu-item>
        <el-menu-item index="/cache">
          <el-icon><Coin /></el-icon>
          <span>缓存管理</span>
        </el-menu-item>
        <el-menu-item index="/query">
          <el-icon><Search /></el-icon>
          <span>DNS 查询</span>
        </el-menu-item>
        <el-menu-item index="/logs">
          <el-icon><List /></el-icon>
          <span>查询日志</span>
        </el-menu-item>
        <el-menu-item index="/listeners">
          <el-icon><Monitor /></el-icon>
          <span>服务监听</span>
        </el-menu-item>
        <el-menu-item index="/settings">
          <el-icon><Setting /></el-icon>
          <span>设置</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <el-container class="main-container">
      <!-- 顶部栏 -->
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
            <el-breadcrumb-item v-if="currentPageTitle">{{ currentPageTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown trigger="click">
            <div class="user-info">
              <el-avatar :size="32" class="user-avatar">
                {{ authStore.username?.charAt(0).toUpperCase() }}
              </el-avatar>
              <span class="username">{{ authStore.username }}</span>
              <el-icon><ArrowDown /></el-icon>
            </div>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="handleLogout">
                  <el-icon><SwitchButton /></el-icon>
                  退出登录
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <!-- 主内容区 -->
      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { ArrowDown, SwitchButton } from '@element-plus/icons-vue'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const activeMenu = computed(() => route.path)

const pageMap: Record<string, string> = {
  '/': '仪表盘',
  '/records': 'DNS 记录',
  '/rewrite': '重写规则',
  '/upstreams': '上游服务器',
  '/cache': '缓存管理',
  '/query': 'DNS 查询',
  '/logs': '查询日志',
  '/listeners': '服务监听',
  '/settings': '设置'
}

const currentPageTitle = computed(() => {
  if (route.path === '/') return ''
  return pageMap[route.path] || ''
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<style scoped>
.layout-container {
  height: 100vh;
}

/* 侧边栏 */
.sidebar {
  background: linear-gradient(180deg, #667eea 0%, #764ba2 100%);
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.15);
  overflow: hidden;
}

.logo {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.logo-icon {
  width: 36px;
  height: 36px;
  filter: brightness(0) invert(1);
}

.logo-text {
  font-size: 20px;
  font-weight: 600;
  color: #fff;
  letter-spacing: 2px;
}

/* 菜单样式 */
.sidebar-menu {
  border-right: none;
  background: transparent;
  padding: 12px 0;
}

.sidebar-menu :deep(.el-menu-item) {
  height: 50px;
  line-height: 50px;
  margin: 4px 12px;
  border-radius: 8px;
  color: rgba(255, 255, 255, 0.8);
  transition: all 0.3s;
}

.sidebar-menu :deep(.el-menu-item:hover) {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.sidebar-menu :deep(.el-menu-item.is-active) {
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
  font-weight: 500;
}

.sidebar-menu :deep(.el-menu-item .el-icon) {
  font-size: 18px;
  margin-right: 8px;
}

/* 主容器 */
.main-container {
  background: #f0f2f5;
}

/* 顶部栏 */
.header {
  height: 64px;
  background: #fff;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
}

.header-left {
  display: flex;
  align-items: center;
}

.header-right {
  display: flex;
  align-items: center;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 8px;
  transition: background 0.3s;
}

.user-info:hover {
  background: #f5f7fa;
}

.user-avatar {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-weight: 600;
}

.username {
  color: #303133;
  font-size: 14px;
}

/* 主内容区 */
.main-content {
  padding: 24px;
  overflow-y: auto;
}

/* 响应式 */
@media (max-width: 768px) {
  .sidebar {
    width: 64px !important;
  }
  
  .logo-text {
    display: none;
  }
  
  .sidebar-menu :deep(.el-menu-item span) {
    display: none;
  }
}
</style>
