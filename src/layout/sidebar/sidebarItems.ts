import { ArrowsJoinIcon, ChecklistIcon, CpuIcon, InfoSquareRoundedIcon, TablerIconComponent } from "vue-tabler-icons"

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
    {
        title: 'Resources Overview',
        icon: ChecklistIcon,
        to: '/resources-overview'
    },
    {
        title: 'Channel Overview',
        icon: ArrowsJoinIcon,
        to: '/channels-overview'
    },
    {
        title: 'Polling Overview',
        icon: ChecklistIcon,
        to: '/polls-overview'
    },
    {
        title: 'CPU Overview',
        icon: CpuIcon,
        to: '/cpu-overview'
    }
]

export default sidebarItems;
