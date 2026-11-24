/* @refresh reload */
import "@fontsource-variable/outfit";
import "./styles/tokens.css";
import "./styles/base.css";
// Theme and locale modules apply <html data-theme> and <html lang> on import, before first paint.
import "./lib/theme";
import "./i18n";
import { render } from "solid-js/web";
import App from "./App";
import { startDeviceSync } from "./lib/devices";

const root = document.getElementById("root");
if (root) {
  render(() => <App />, root);
}

startDeviceSync().catch((error: unknown) => {
  console.error("Could not load the TV list", error);
});
