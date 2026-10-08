/* @refresh reload */
import { render } from "solid-js/web";
import "./index.css";

async function main() {
  // Outside the desktop app (plain browser during development), fake the backend
  // before any module that talks to it is loaded.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMocks } = await import("./dev/mock");
    installMocks();
  }
  const { default: App } = await import("./App");
  render(() => <App />, document.getElementById("root") as HTMLElement);
}

void main();
