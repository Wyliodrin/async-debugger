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
    ]
}

export default MainRoutes;