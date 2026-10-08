import { mount } from "svelte";
import "leaflet/dist/leaflet.css";
import "@fontsource/barlow/latin-400.css";
import "@fontsource/barlow/latin-500.css";
import "@fontsource/barlow/latin-600.css";
import "@fontsource/barlow/latin-700.css";
import "./app.css";
import { applyStoredTheme } from "./lib/theme";
import App from "./App.svelte";

applyStoredTheme();

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
