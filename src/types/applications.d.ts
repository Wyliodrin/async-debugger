import { AppConnStatus } from "@/types/appConnStatus";

export interface Application {
  conn_status: AppConnStatus;
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