import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createTheme, MantineProvider } from "@mantine/core";
import "@mantine/core/styles.css";
import "./styles.css";
import App from "./App";

const theme = createTheme({
  primaryColor: "cyan",
  defaultRadius: "sm",
  fontFamily: "Segoe UI, system-ui, sans-serif",
  headings: {
    fontFamily: "Segoe UI, system-ui, sans-serif",
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MantineProvider defaultColorScheme="dark" theme={theme}>
      <App />
    </MantineProvider>
  </StrictMode>,
);
