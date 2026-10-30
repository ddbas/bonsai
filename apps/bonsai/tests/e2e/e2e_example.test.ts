import { expect, it } from "vitest";
import { startGenericContainer } from "./support";

/**
 * Canary for the whole e2e infrastructure: if this passes, Docker is
 * reachable and testcontainers works.
 */
it("container starts and stops", async () => {
  const container = await startGenericContainer();
  expect(container.getId()).not.toBe("");
  await container.stop();
});
