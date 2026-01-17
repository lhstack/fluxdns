<template>
  <el-container class="layout-container">
    <!-- PC 端侧边栏 -->
    <el-aside 
      v-if="!isMobile"
      :width="isCollapse ? '64px' : '240px'" 
      class="sidebar"
    >
      <!-- Logo -->
      <div class="logo">
        <div class="logo-icon">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" fill="#667eea"/>
            <path d="M12 32 Q22 22, 32 32 T52 32" stroke="#fff" stroke-width="4" fill="none" stroke-linecap="round"/>
            <path d="M12 40 Q22 30, 32 40 T52 40" stroke="#fff" stroke-width="3" fill="none" stroke-linecap="round" opacity="0.7"/>
            <path d="M12 24 Q22 34, 32 24 T52 24" stroke="#fff" stroke-width="3" fill="none" stroke-linecap="round" opacity="0.7"/>
            <circle cx="16" cy="32" r="3" fill="#fff"/>
            <circle cx="48" cy="32" r="3" fill="#fff"/>
          </svg>
        </div>
        <span v-if="!isCollapse" class="logo-text">FluxDNS</span>
      </div>

      <!-- 导航菜单 -->
      <el-menu
        :default-active="activeMenu"
        :collapse="isCollapse"
        router
        class="sidebar-menu"
      >
        <el-menu-item v-for="item in menuItems" :key="item.path" :index="item.path">
          <el-icon><component :is="item.icon" /></el-icon>
          <template #title><span>{{ item.label }}</span></template>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <!-- 移动端抽屉导航 -->
    <el-drawer
      v-model="mobileMenuVisible"
      direction="ltr"
      size="240px"
      :with-header="false"
      class="mobile-drawer"
    >
      <div class="sidebar mobile-sidebar">
        <div class="logo">
          <span class="logo-text">FluxDNS</span>
        </div>
        <el-menu
          :default-active="activeMenu"
          router
          @select="mobileMenuVisible = false"
          class="sidebar-menu"
        >
          <el-menu-item v-for="item in menuItems" :key="item.path" :index="item.path">
            <el-icon><component :is="item.icon" /></el-icon>
            <span>{{ item.label }}</span>
          </el-menu-item>
        </el-menu>
      </div>
    </el-drawer>

    <el-container class="main-container">
      <!-- 顶部栏 -->
      <el-header class="header">
        <div class="header-left">
          <div class="menu-toggle" @click="toggleMenu">
             <el-icon :size="20"><Expand v-if="isCollapse || isMobile" /><Fold v-else /></el-icon>
          </div>
          <el-breadcrumb separator="/" class="hidden-xs-only">
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
              <span class="username hidden-xs-only">{{ authStore.username }}</span>
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
        <div class="content-wrapper">
          <router-view v-slot="{ Component }">
            <transition name="fade-transform" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </div>
      </el-main>
    </el-container>
    
    <!-- AI 助手浮动组件 -->
    <AiAssistant />
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { 
  ArrowDown, SwitchButton, Odometer, Document, Edit, 
  Connection, Coin, Search, List, Monitor, Setting,
  Expand, Fold, ChatDotRound
} from '@element-plus/icons-vue'
import AiAssistant from '../components/AiAssistant.vue'
import { useResponsive } from '../composables/useResponsive'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const { isMobile, isCollapse } = useResponsive()

const mobileMenuVisible = ref(false)

const activeMenu = computed(() => route.path)

const menuItems = [
  { path: '/', label: '仪表盘', icon: Odometer },
  { path: '/records', label: 'DNS 记录', icon: Document },
  { path: '/rewrite', label: '重写规则', icon: Edit },
  { path: '/upstreams', label: '上游服务器', icon: Connection },
  { path: '/cache', label: '缓存管理', icon: Coin },
  { path: '/query', label: 'DNS 查询', icon: Search },
  { path: '/logs', label: '查询日志', icon: List },
  { path: '/listeners', label: '服务监听', icon: Monitor },
  { path: '/settings', label: '设置', icon: Setting },
  { path: '/llm', label: 'AI 助手', icon: ChatDotRound },
]

const pageMap = menuItems.reduce((acc, item) => {
  acc[item.path] = item.label
  return acc
}, {} as Record<string, string>)

const currentPageTitle = computed(() => {
  if (route.path === '/') return ''
  return pageMap[route.path] || ''
})

const toggleMenu = () => {
  if (isMobile.value) {
    mobileMenuVisible.value = !mobileMenuVisible.value
  } else {
    isCollapse.value = !isCollapse.value
  }
}

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
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  flex-direction: column;
}

.logo {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
}

.logo-icon {
  width: 32px;
  height: 32px;
}

.logo-icon svg {
  width: 100%;
  height: 100%;
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
  flex: 1;
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

/* 折叠状态下的图标居中 */
.sidebar-menu.el-menu--collapse :deep(.el-menu-item) {
  margin: 4px 8px;
  padding: 0 !important;
  display: flex;
  justify-content: center;
}

.sidebar-menu.el-menu--collapse :deep(.el-menu-item .el-icon) {
  margin-right: 0;
}

/* 主容器 */
.main-container {
  background: #f0f2f5;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
  z-index: 10;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.menu-toggle {
  cursor: pointer;
  padding: 8px;
  border-radius: 4px;
  color: #667eea;
  background: #f0f2f5;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
}

.menu-toggle:hover {
  background: #e4e7ed;
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
  flex: 1;
}

.content-wrapper {
  max-width: 1400px;
  margin: 0 auto;
}

/* 移动端侧边栏覆盖 */
:deep(.mobile-drawer) .el-drawer__body {
  padding: 0;
}

.mobile-sidebar {
  height: 100%;
  width: 100% !important;
  background: linear-gradient(180deg, #667eea 0%, #764ba2 100%) !important;
}

.mobile-sidebar .logo-text {
  display: block !important;
}

/* 响应式辅助 */
@media (max-width: 768px) {
  .hidden-xs-only {
    display: none !important;
  }
  .main-content {
    padding: 16px;
  }
}

/* 过渡动画 */
.fade-transform-enter-active,
.fade-transform-leave-active {
  transition: all 0.3s;
}

.fade-transform-enter-from {
  opacity: 0;
  transform: translateX(-15px);
}

.fade-transform-leave-to {
  opacity: 0;
  transform: translateX(15px);
}
</style>
