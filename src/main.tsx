import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { DiagProvider } from "./lib/store";
import { TooltipProvider } from "./components/ui/tooltip";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <TooltipProvider>
      <DiagProvider>
        <App />
      </DiagProvider>
    </TooltipProvider>
  </StrictMode>,
);
