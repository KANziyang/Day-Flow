import { createRoot } from "react-dom/client";
import { DataProvider } from "./hooks/useData";
import { App } from "./App";
import { WidgetView } from "./widget/WidgetView";
import { BallView } from "./widget/BallView";
import "./styles/app.css";
const surface = new URLSearchParams(location.search).get("window");
document.documentElement.classList.toggle("is-widget", surface === "widget" || surface === "ball");
document.documentElement.classList.toggle("is-ball", surface === "ball");
createRoot(document.getElementById("root")!).render(
  <DataProvider>{surface === "widget" ? <WidgetView /> : surface === "ball" ? <BallView /> : <App />}</DataProvider>,
);
