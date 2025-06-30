export type Task = {
    app_id: string,
    id: number;
    tid?: number;
    name?: string;
    kind: string;
    location: TaskLocation;
    stats_info: StatsInfo;
};

export type StatsInfo = {
    runtime?: TaskTime;
    busy?: TaskTime;
    scheduled?: TaskTime;
    idle?: TaskTime;
}

export type TaskTime = {
    hours: number;
    minutes: number;
    seconds: number;
}

export type TaskLocation = {
    file: string | null;
    module_path: string | null;
    line: number | null;
    column: number | null;
}
