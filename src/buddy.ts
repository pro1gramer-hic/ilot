// Mochi's window outside the island: a separate page with its own capability
// (capabilities/buddy.json), transparent, never focused, never clicked.
import "./lib/tokens.css";
import { mount } from "svelte";
import Buddy from "./Buddy.svelte";

mount(Buddy, { target: document.getElementById("app")! });
