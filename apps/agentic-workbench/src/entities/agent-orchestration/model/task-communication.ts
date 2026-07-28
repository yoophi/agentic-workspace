import type {
  CoordinatorNotification,
  TaskCommand,
} from "./types";

export type TaskCommunicationState = {
  commands: Record<string, TaskCommand>;
  notifications: Record<string, CoordinatorNotification>;
};

export type TaskCommunicationAction =
  | { type: "hydrate"; commands: TaskCommand[]; notifications: CoordinatorNotification[] }
  | { type: "commandUpdated"; command: TaskCommand }
  | { type: "notificationUpdated"; notification: CoordinatorNotification };

export const initialTaskCommunicationState: TaskCommunicationState = {
  commands: {},
  notifications: {},
};

export function taskCommunicationReducer(
  state: TaskCommunicationState,
  action: TaskCommunicationAction,
): TaskCommunicationState {
  if (action.type === "hydrate") {
    return {
      commands: Object.fromEntries(action.commands.map((command) => [command.id, command])),
      notifications: Object.fromEntries(
        action.notifications.map((notification) => [
          notification.id,
          notification,
        ]),
      ),
    };
  }
  if (action.type === "commandUpdated") {
    const previous = state.commands[action.command.id];
    if (previous && previous.updatedAt > action.command.updatedAt) return state;
    return {
      ...state,
      commands: { ...state.commands, [action.command.id]: action.command },
    };
  }
  const previous = state.notifications[action.notification.id];
  if (previous && previous.updatedAt > action.notification.updatedAt) return state;
  return {
    ...state,
    notifications: {
      ...state.notifications,
      [action.notification.id]: action.notification,
    },
  };
}

export function latestTaskCommand(
  state: TaskCommunicationState,
  taskId: string,
  kind?: TaskCommand["kind"],
) {
  return Object.values(state.commands)
    .filter((command) => command.taskId === taskId && (!kind || command.kind === kind))
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))[0] ?? null;
}
