import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { loadAppFonts } from "./lib/fonts";
import "./styles.css";

loadAppFonts();

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
