import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { DiagProvider } from "./lib/store";
import { TooltipProvider } from "./components/ui/tooltip";
import "./index.css";

// iPadOS reports itself as a Mac; touch points tell them apart.
if (/iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)) {
  document.documentElement.classList.add("ios");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <TooltipProvider>
      <DiagProvider>
        <App />
      </DiagProvider>
    </TooltipProvider>
  </StrictMode>,
);

