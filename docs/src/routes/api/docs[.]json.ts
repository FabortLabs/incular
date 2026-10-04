import { createFileRoute } from "@tanstack/react-router";
import { getDocsData } from "@/lib/source";

export const Route = createFileRoute("/api/docs.json")({
  server: {
    handlers: {
      GET: async () => Response.json(await getDocsData()),
    },
  },
});
