import type { Preview } from "@storybook/react-vite";
// Storybook 전용 초기화: 프로덕션 Tauri 앱 번들에는 React Grab을 포함하지 않는다.
import "react-grab";
import "../src/app/styles.css";

const preview: Preview = {
  parameters: {
    backgrounds: { default: "hushline", values: [{ name: "hushline", value: "#0b0d0a" }] },
    controls: { matchers: { color: /(background|color)$/i, date: /Date$/i } },
    layout: "padded",
  },
  tags: ["autodocs"],
};

export default preview;
