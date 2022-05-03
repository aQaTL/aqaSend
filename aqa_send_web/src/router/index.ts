import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import Files from '../views/Files.vue'

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    name: 'Files',
    component: Files
  },
  {
    path: '/upload',
    name: 'Upload',
    // route level code-splitting
    // this generates a separate chunk (about.[hash].js) for this route
    // which is lazy-loaded when the route is visited.
    component: () => import(/* webpackChunkName: "upload" */ '../views/Upload.vue')
  },
  {
    path: '/account',
    name: 'Account',
    component: () => import(/* webpackChunkName: "account" */ '../views/Account.vue')
  }
]

const router = createRouter({
  history: createWebHistory(process.env.BASE_URL),
  routes
})

export default router
