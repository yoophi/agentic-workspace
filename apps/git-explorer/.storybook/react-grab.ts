import { commentPlugin, registerPlugin, unregisterPlugin } from "react-grab";

unregisterPlugin(commentPlugin.name);
registerPlugin(commentPlugin);
