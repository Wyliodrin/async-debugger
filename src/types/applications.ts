export type Application = {
  id: string;
  title: string;
  url: string;
  state: ApplicationState;

  startTime?: string,
  pid?: number,
  cpuUsage?: number,
  memoryUsage?: number,
};

export enum ApplicationState {
  Enabled = "Enabled",
  Disabled = "Disabled"
};
