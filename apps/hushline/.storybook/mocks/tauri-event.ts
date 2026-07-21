export type UnlistenFn = () => void;
export type Event<T> = { event: string; id: number; payload: T };
export type EventCallback<T> = (event: Event<T>) => void;
export async function listen<T>(_event: string, _handler: EventCallback<T>): Promise<UnlistenFn> { return () => {}; }
