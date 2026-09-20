import * as React from "react";
import * as ReactDOM from "react-dom/client";
import App from "./App";
import "./components/Tokens.css";
import "./index.css";

// uPlot global stylesheet — static optional; inline fallback if not resolved at build
import "uplot/dist/uPlot.min.css";

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("Missing #root element");

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
