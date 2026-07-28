import { listen } from "@tauri-apps/api/event";
import avatarSvg from "./assets/luozi-orbit-avatar.svg?raw";
import {
  initialHudState,
  reduceHudState,
  type HudEnergyEvent,
  type HudPhaseEvent,
} from "./overlay-model";
import "./overlay.css";

const root = document.querySelector<HTMLElement>(".overlay")!;
const avatar = document.querySelector<HTMLElement>("#avatar")!;
const title = document.querySelector<HTMLElement>(".title")!;
const detail = document.querySelector<HTMLElement>(".detail")!;

let state = initialHudState;
avatar.innerHTML = avatarSvg;

function render() {
  root.dataset.state = state.kind;
  title.textContent = state.title;
  detail.textContent = state.detail;
  detail.hidden = state.detail.length === 0;

  const svg = avatar.querySelector<SVGElement>("svg");
  const energy = state.energy;
  svg?.style.setProperty("--energy", energy.toFixed(3));
  svg?.style.setProperty("--orbit-dash", `${(0.18 + energy * 0.42).toFixed(3)} 1`);
  svg?.style.setProperty("--orbit-opacity", (0.28 + energy * 0.72).toFixed(3));
  svg?.style.setProperty(
    "--orbit-back-opacity",
    (0.18 + energy * 0.46).toFixed(3),
  );
}

void listen<HudPhaseEvent>("session://phase", (event) => {
  state = reduceHudState(state, event.payload);
  render();
});

void listen<HudEnergyEvent>("session://energy", (event) => {
  state = reduceHudState(state, event.payload);
  render();
});

render();
