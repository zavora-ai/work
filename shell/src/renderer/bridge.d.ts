import type { StudioBridge } from "../shared/bridge.ts";

declare global {
  interface Window {
    studio: StudioBridge;
  }
}
