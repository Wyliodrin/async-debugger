import { Resource } from "@/types/resources";
import { Task } from "@/types/tasks";
import { Poll } from "@/types/polls";
import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export const useDataStore = defineStore('data', () => {
    const pause = ref(false);

    function togglePause(){
        pause.value = !pause.value;
    }

    const tasks= ref<Task[]>([]);

    function handleTaskUpdate(e: {payload: any[]}) {
        if (pause.value == false) {
            tasks.value = e.payload.map(t => {
            const formatted = {
                runtime:   t.runtime.formatted,
                scheduled: t.scheduled.formatted,
                idle:      t.idle.formatted,
                busy:      t.busy.formatted,
            }

            let stateKey: string
            if (typeof t.state === 'string') {
                stateKey = t.state
            } else {
                const keys = Object.keys(t.state ?? {})
                stateKey = keys.length ? keys[0]! : 'Unknown'
            }

            return {
                ...t,
                state: stateKey,
                ...formatted,
            }
            })
        }
    }

    async function editTask(task_id: number, task_name: string, task_color:string, app_name: string) {
        await invoke('edit_task', {taskId: task_id, taskName: task_name, taskColor: task_color, appName: app_name}).then(
            () => {
                console.log("numele task-ului modificat cu succes");
            }
        ).catch(
            (error) => {
                console.log(task_id.toString() + "nu s-a putut modifica numele task-ului eroare:" + error);
            }
        );
    }

    const resources = ref<Resource[]>([]);

    function handleResourceUpdate(e: {payload: any[]}) {
        if (pause.value == false) {
            resources.value = e.payload.map(r => {
                return {
                ...r,
                duration: r.duration.formatted
                }
            })
            console.log("Resources primite:" + JSON.stringify(e.payload[0]));
        }
    }

    const polls = ref<Poll[]>([]);
    
    function handlePollUpdate(e: {payload: any[]}) {
        if (pause.value == false) {
            polls.value = e.payload.map(p => {
                return {
                    ...p
                }
            })
            console.log("Polls primite: " + JSON.stringify(e.payload[0]));
        }
    }


    return {pause, togglePause, tasks, handleTaskUpdate, resources,
        handleResourceUpdate, polls, handlePollUpdate, editTask,
    };
});