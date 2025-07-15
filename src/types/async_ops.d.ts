export interface TimeStamp {
  seconds: number;
  nanos: number;
}

export interface CPUOverview {
  started_at: TimeStamp | null;
  stopped_at: TimeStamp | null;
  resource_target: string | null;
}

export interface TaskOp {
  task_id: number;
  operations: CPUOverview[];
}

export type TaskOpMap = Record<string, TaskOp>;
