import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router";
import App from "./App";
import OverlayApp from "./Overlay";
import "./styles.css";

declare global {
  interface Window {
    __GB_OVERLAY__?: boolean;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {window.__GB_OVERLAY__ ? (
      <OverlayApp />
    ) : (
      <HashRouter>
        <App />
      </HashRouter>
    )}
  </React.StrictMode>,
);
