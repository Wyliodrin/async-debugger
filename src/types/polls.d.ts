export type Poll = {
    received_at: string,
    app_name: string,
    poll_type: string,
    resource_id: number,
    task_id: number,
    is_ready: boolean,
    location: string,
}