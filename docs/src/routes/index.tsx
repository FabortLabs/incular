import { createFileRoute } from "@tanstack/react-router";
import { LandingPage } from "@/components/landing-page";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "Incular — Native apps. Unbound ideas." },
      {
        name: "description",
        content:
          "Bring your ideas to life with Incular, a declarative native Rust UI framework. Composable widgets, reactive state, and one shared desktop host for Windows, macOS, and Linux.",
      },
    ],
  }),
  component: LandingPage,
});
