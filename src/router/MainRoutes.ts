const MainRoutes = {
    path: '/main',
    meta: {
        requiresAuth: false
    },
    redirect: 'main',
    component: () => import('@/layout/AppLayout.vue'),
    children: [
        {
            path: '/',
            redirect: "/system-information",
        },
        {
            name: 'System Information',
            path: '/system-information',
            component: () => import('@/views/SystemInformation.vue')
        },
        {
            name: 'Tasks Overview',
            path: '/tasks-overview',
            component: () => import('@/views/Tasks.vue')
        },
        {
            name: 'Resources Overview',
            path: '/resources-overview',
            component: () => import('@/views/Resources.vue')
        },
        {
            name: 'Channels Overview',
            path: '/channels-overview',
            component: () => import('@/views/Channels.vue')
        },
        {
            name: 'Polling Overview',
            path: '/polls-overview',
            component: () => import('@/views/Polls.vue')
        },
        {
            name: 'CPU Overview',
            path: '/cpu-overview',
            component: () => import('@/views/CPU.vue')
        }
    ]
}

export default MainRoutes;