export interface SpyPayload {
  id: string;
  event: {
    variant: string;
    data?:   unknown;
  };
  timestamp?: string;
}
