import React from "react";
import { createRoot } from "react-dom/client";
import { bootstrapQaPlatform } from "@rightkit/platform-ui";
import { App } from "./App";
import { loadAppFonts } from "./lib/fonts";
import "./styles.css";

loadAppFonts();
bootstrapQaPlatform({
  query: window.location.search,
  env: {
    RIGHTKIT_QA: import.meta.env.VITE_CUTRIGHT_QA,
    RIGHTKIT_QA_PLATFORM: import.meta.env.VITE_RIGHTKIT_QA_PLATFORM,
  },
});

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
