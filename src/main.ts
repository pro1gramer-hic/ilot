import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/pixelify-sans/latin-600.css";
import "./lib/tokens.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { start } from "./lib/startup";

mount(App, { target: document.getElementById("app")! });
void start();
