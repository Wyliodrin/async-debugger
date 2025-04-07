import { createVuetify } from "vuetify";
import type { ThemeDefinition } from "vuetify";
import "vuetify/styles";
import "@mdi/font/css/materialdesignicons.css";
import * as components from "vuetify/components";
import * as directives from "vuetify/directives";

// Define light theme
const lightTheme: ThemeDefinition = {
    dark: false,
    colors: {
        primary: "#d35400",
        secondary: "#8e44ad",
        background: "#ecf0f1",
        error: "#c0392b",
        info: "#2980b9",
        success: "#27ae60",
        warning: "#f1c40f",
    },
};

// Define dark theme
const darkTheme: ThemeDefinition = {
    dark: true,
    colors: {
        primary: "#d35400",
        secondary: "#8e44ad",
        background: "#2f3640",
        error: "#c0392b",
        info: "#2980b9",
        success: "#27ae60",
        warning: "#f1c40f",
    },
};

export default createVuetify({
    components,
    directives,
    theme: {
        defaultTheme: 'light',
        themes: {
            light: lightTheme,
            dark: darkTheme,
        },
    },
    icons: {
        defaultSet: 'mdi',
    },
});
