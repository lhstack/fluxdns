import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/Login.vue'),
    meta: { requiresAuth: false }
  },
  {
    path: '/',
    component: () => import('../layouts/MainLayout.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue')
      },
      {
        path: 'records',
        name: 'DnsRecords',
        component: () => import('../views/DnsRecords.vue')
      },
      {
        path: 'rewrite',
        name: 'RewriteRules',
        component: () => import('../views/RewriteRules.vue')
      },
      {
        path: 'upstreams',
        name: 'Upstreams',
        component: () => import('../views/Upstreams.vue')
      },
      {
        path: 'cache',
        name: 'Cache',
        component: () => import('../views/Cache.vue')
      },
      {
        path: 'query',
        name: 'DnsQuery',
        component: () => import('../views/DnsQuery.vue')
      },
      {
        path: 'logs',
        name: 'QueryLogs',
        component: () => import('../views/QueryLogs.vue')
      },
      {
        path: 'listeners',
        name: 'Listeners',
        component: () => import('../views/Listeners.vue')
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../views/Settings.vue')
      },
      {
        path: 'llm',
        name: 'LlmSettings',
        component: () => import('../views/LlmSettings.vue')
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

// Navigation guard for authentication
router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('token')

  if (to.meta.requiresAuth && !token) {
    next('/login')
  } else if (to.path === '/login' && token) {
    next('/')
  } else {
    next()
  }
})

export default router
