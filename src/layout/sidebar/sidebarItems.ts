import { ChecklistIcon, InfoSquareRoundedIcon, TablerIconComponent } from "vue-tabler-icons"

export interface sidebarItem {
    title: string,
    icon: TablerIconComponent,
    to: string
}

const sidebarItems: sidebarItem[] = [
    {
        title: 'System Information',
        icon: InfoSquareRoundedIcon,
        to: '/system-information'
    },
    {
        title: 'Tasks Overview',
        icon: ChecklistIcon,
        to: '/tasks-overview'
    },
]

export default sidebarItems;
