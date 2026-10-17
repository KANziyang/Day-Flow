import { createRoot } from "react-dom/client";
import { DataProvider } from "./hooks/useData";
import { App } from "./App";
import { WidgetView } from "./widget/WidgetView";
import "./styles/app.css";
const widget = new URLSearchParams(location.search).get("window") === "widget";
document.documentElement.classList.toggle("is-widget", widget);
createRoot(document.getElementById("root")!).render(
  <DataProvider>{widget ? <WidgetView /> : <App />}</DataProvider>,
);
