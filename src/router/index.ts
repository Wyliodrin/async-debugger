import { createMemoryHistory, createRouter } from 'vue-router';
import MainRoutes from './MainRoutes';

export const router = createRouter({
    history: createMemoryHistory(),
    routes: [
        MainRoutes,
    ]
});