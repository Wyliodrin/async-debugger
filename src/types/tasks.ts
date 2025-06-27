export type Task = {
    app_id: string,
    id: number;
    tid?: number;
    name?: string;
    kind: string;
};

export enum TaskState {
  SPAWN = "SPAWN",
  BLOCKING = "BLOCKING"
};
