export type Application = {
  id: string;
  title: string;
  url: string;
  state: string;

  startTime?: string,
  pid?: number,
  cpuUsage?: number,
  memoryUsage?: number,
}
