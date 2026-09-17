import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { MdGuideWindow } from "./features/MdGuide/MdGuideWindow";
import { MD_GUIDE_WINDOW_LABEL } from "./lib/mdGuideWindow";
import { PdfExportWindow } from "./features/PdfExport/PdfExportWindow";
import { PDF_EXPORT_WINDOW_LABEL } from "./features/PdfExport/pdfExportProtocol";

const isMdGuideWindow = getCurrentWindow().label === MD_GUIDE_WINDOW_LABEL;
const isPdfExportWindow = getCurrentWindow().label === PDF_EXPORT_WINDOW_LABEL;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isPdfExportWindow ? <PdfExportWindow /> : isMdGuideWindow ? <MdGuideWindow /> : <App />}
  </React.StrictMode>,
);
