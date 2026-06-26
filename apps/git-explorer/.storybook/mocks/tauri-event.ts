type Listener<T> = (event: { payload: T }) => void;

const listeners = new Map<string, Set<Listener<unknown>>>();

export async function listen<T>(eventName: string, callback: Listener<T>) {
  const eventListeners = listeners.get(eventName) ?? new Set<Listener<unknown>>();
  eventListeners.add(callback as Listener<unknown>);
  listeners.set(eventName, eventListeners);

  return () => {
    eventListeners.delete(callback as Listener<unknown>);
  };
}

export function emitMockEvent(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((callback) => callback({ payload }));
}
