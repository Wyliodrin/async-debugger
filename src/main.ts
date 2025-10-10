import { createApp } from "vue";
import { createPinia } from 'pinia';
import App from "./App.vue";
import vuetify from './plugins/vuetify';
import VueTablerIcons from 'vue-tabler-icons';
import { router } from './router';
import './styles/timestamps.css';

const app = createApp(App);
app.use(router);
app.use(createPinia());
app.use(VueTablerIcons);
app.use(vuetify).mount("#app");
