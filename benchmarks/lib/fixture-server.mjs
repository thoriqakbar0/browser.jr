import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const FIXTURE_DIRECTORY = join(dirname(fileURLToPath(import.meta.url)), "..", "fixtures");
const ROUTES = new Map([
  ["/", "index.html"],
  ["/actionability.html", "actionability.html"],
  ["/index.html", "index.html"],
  ["/next.html", "next.html"],
]);

export async function startFixtureServer() {
  const bodies = new Map();
  for (const file of new Set(ROUTES.values())) {
    bodies.set(file, await readFile(join(FIXTURE_DIRECTORY, file)));
  }

  const server = createServer((request, response) => {
    const pathname = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
    const file = ROUTES.get(pathname);
    if (!file) {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("not found\n");
      return;
    }
    const body = bodies.get(file);
    response.writeHead(200, {
      "cache-control": "no-store",
      "content-length": body.length,
      "content-type": "text/html; charset=utf-8",
    });
    response.end(body);
  });

  await new Promise((resolvePromise, rejectPromise) => {
    server.once("error", rejectPromise);
    server.listen(0, "127.0.0.1", resolvePromise);
  });
  const address = server.address();
  if (!address || typeof address === "string") {
    server.close();
    throw new Error("fixture server did not receive a TCP address");
  }

  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    close: () =>
      new Promise((resolvePromise, rejectPromise) => {
        server.close((error) => (error ? rejectPromise(error) : resolvePromise()));
      }),
  };
}
