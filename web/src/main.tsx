import * as React from "react";
import * as ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./fonts.css";
import "./components/Tokens.css";
import "./styles.css";

const el = document.getElementById("root");
if (!el) throw new Error("Missing #root");
ReactDOM.createRoot(el).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>,
);
