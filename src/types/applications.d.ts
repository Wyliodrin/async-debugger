export interface Application {
  connection_status: string;
  state: string,
  pid: number;
  id: string;
  startTime: string;
  title: string;
  url: string;
  cpu_usage: number;
  memory_usage: number;
  processStatus: string;
}