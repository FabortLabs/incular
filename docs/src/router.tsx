import { createRouter as createTanStackRouter } from "@tanstack/react-router";
import { NotFound } from "@/components/not-found";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  return createTanStackRouter({
    routeTree,
    basepath: import.meta.env.BASE_URL,
    defaultPreload: "intent",
    scrollRestoration: true,
    defaultNotFoundComponent: NotFound,
  });
}
