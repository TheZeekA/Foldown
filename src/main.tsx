import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { MdGuideWindow } from "./features/MdGuide/MdGuideWindow";
import { MD_GUIDE_WINDOW_LABEL } from "./lib/mdGuideWindow";

const isMdGuideWindow = getCurrentWindow().label === MD_GUIDE_WINDOW_LABEL;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isMdGuideWindow ? <MdGuideWindow /> : <App />}
  </React.StrictMode>,
);
