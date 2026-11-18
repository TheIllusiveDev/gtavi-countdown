import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./app.css";

const target = document.getElementById("app");
if (!target) throw new Error("Mount point #app is missing from index.html.");

createRoot(target).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
