export type Task = {
    app_name: string,
    id: number,
    tid?: number,
    name?: string,
    kind: string,
    state: string,
    runtime: string,
    scheduled: string,
    idle: string,
    busy: string,
    location: string
};
