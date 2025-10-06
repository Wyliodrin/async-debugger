export type Task = {
    app_name: string,
    id: number,
    tid: number,
    name: string,
    color: string,
    kind: string,
    state: string,
    runtime: string,
    scheduled: string,
    idle: string,
    busy: string,
    location: string,
    created_at: string,
    warnings: TaskWarnings
};

type SelfWakePercent = {
  enabled: boolean;
  parameter: number;
  description: string;
};

type LostWaker = {
  enabled: boolean;
  parameter?: number,
};

type NeverYielded = {
  enabled: boolean;
  parameter: number;
  description: string;
};

type AutoBoxedFuture = {
  enabled: boolean;
  parameter?: number,
};

type LargeFuture = {
  enabled: boolean;
  parameter: number;
  description: string;
};

type TaskWarnings = {
  self_wake_percent: SelfWakePercent;
  lost_waker: LostWaker;
  never_yielded: NeverYielded;
  auto_boxed_feature: AutoBoxedFuture;
  large_feature: LargeFuture;
};
