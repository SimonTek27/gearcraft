/**
 * Minimal app state shared across routes.
 */

import type { DeviceInfo, Route } from "./types";

let selectedDevice: DeviceInfo | null = null;
let pendingRoute: Route | null = null;

export function getSelectedDevice(): DeviceInfo | null {
  return selectedDevice;
}

export function setSelectedDevice(device: DeviceInfo | null) {
  selectedDevice = device;
}

export function consumePendingRoute(): Route | null {
  const r = pendingRoute;
  pendingRoute = null;
  return r;
}

export function requestNavigate(route: Route) {
  pendingRoute = route;
}
