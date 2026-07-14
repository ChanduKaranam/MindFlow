import React from "react";
import ReactDOM from "react-dom/client";
import Scratchpad from "./Scratchpad";
import "@/i18n";
import "../App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Scratchpad />
  </React.StrictMode>,
);
