import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createConnection } from "node:net";
import { mkdtemp, writeFile, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const CLI = resolve("plugin/cli.mjs");
const PROTOCOL = "agent-browser.plugin.v1";

function runPlugin(request, options = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(process.execPath, [CLI], {
      env: { ...process.env, ...options.env },
      stdio: ["pipe", "pipe", "pipe"],
    });
    const stdout = [];
    const stderr = [];
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.once("error", rejectPromise);
    child.once("close", (code) => {
      resolvePromise({
        code,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      });
    });
    child.stdin.end(`${JSON.stringify(request)}\n`);
  });
}

function runCli(args) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(process.execPath, [CLI, ...args], { stdio: ["ignore", "pipe", "pipe"] });
    const stdout = [];
    const stderr = [];
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.once("error", rejectPromise);
    child.once("close", (code) => resolvePromise({
      code,
      stdout: Buffer.concat(stdout).toString("utf8"),
      stderr: Buffer.concat(stderr).toString("utf8"),
    }));
  });
}

async function fakeBrowserJr(directory) {
  const path = join(directory, "browser-jr-fake.mjs");
  await writeFile(path, `#!/usr/bin/env node
import { createInterface } from "node:readline";
const args = process.argv.slice(2);
if (!args.includes("session")) process.exit(2);
const json = args.includes("--json");
if (!json) process.exit(3);
const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
let sequence = 0;
console.log(JSON.stringify({success:true,error:null,data:{event:"ready"}}));
for await (const command of lines) {
  sequence += 1;
  console.log(JSON.stringify({success:true,error:null,data:{event:"command",sequence,output:command === "exit" ? "" : \`ran:\${command}\`}}));
  if (command === "exit") break;
}
console.log(JSON.stringify({success:true,error:null,data:{event:"closed"}}));
`);
  await chmod(path, 0o755);
  return path;
}

function pluginRequest(type, request = {}) {
  return { protocol: PROTOCOL, type, capability: type, request };
}

test("manifest declares the command capabilities", async () => {
  const result = await runPlugin(pluginRequest("plugin.manifest"));
  assert.equal(result.code, 0);
  assert.equal(result.stderr, "");
  const lines = result.stdout.trim().split("\n");
  assert.equal(lines.length, 1);
  const response = JSON.parse(lines[0]);
  assert.deepEqual(response, {
    protocol: PROTOCOL,
    success: true,
    manifest: {
      name: "browser-jr",
      capabilities: ["command.run", "browserjr.session", "browserjr.command"],
    },
  });
});

test("session runs a bounded command batch and appends exit", async () => {
  const directory = await mkdtemp(join(tmpdir(), "browser-jr-plugin-test-"));
  try {
    const binary = await fakeBrowserJr(directory);
    const result = await runPlugin(
      pluginRequest("browserjr.session", {
        commands: ["open https://example.com", "get title"],
        binary,
      }),
    );
    assert.equal(result.code, 0);
    const response = JSON.parse(result.stdout);
    assert.equal(response.success, true);
    assert.deepEqual(
      response.result.events
        .filter((event) => event.data?.event === "command")
        .map((event) => event.data.output),
      ["ran:open https://example.com", "ran:get title", ""],
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("session rejects command line injection", async () => {
  const result = await runPlugin(
    pluginRequest("browserjr.session", { commands: ["open https://example.com\nexit"] }),
  );
  assert.equal(result.code, 0);
  const response = JSON.parse(result.stdout);
  assert.equal(response.success, false);
  assert.match(response.error, /line breaks/);
});

test("relay client rejects non-loopback hosts", async () => {
  const result = await runPlugin(
    pluginRequest("browserjr.command", {
      host: "example.com",
      port: 443,
      token: "token",
      command: "get title",
    }),
  );
  assert.equal(result.code, 0);
  const response = JSON.parse(result.stdout);
  assert.equal(response.success, false);
  assert.match(response.error, /loopback host/);
});

test("relay command rejects line injection before connecting", async () => {
  const result = await runPlugin(
    pluginRequest("browserjr.command", {
      port: 1,
      token: "token",
      command: "get title\nexit",
    }),
  );
  assert.equal(result.code, 0);
  const response = JSON.parse(result.stdout);
  assert.equal(response.success, false);
  assert.match(response.error, /line breaks/);
});

test("serve reports a missing native binary as one JSON failure", async () => {
  const result = await runCli(["serve", "--binary", "/definitely/missing/browser-jr"]);
  assert.equal(result.code, 1);
  assert.equal(result.stderr, "");
  const lines = result.stdout.trim().split("\n");
  assert.equal(lines.length, 1);
  const response = JSON.parse(lines[0]);
  assert.equal(response.success, false);
  assert.match(response.error, /ENOENT/);
});

test("session timeout force-kills a signal-ignoring native process", async () => {
  const directory = await mkdtemp(join(tmpdir(), "browser-jr-plugin-timeout-test-"));
  try {
    const binary = join(directory, "browser-jr-hung.mjs");
    await writeFile(binary, `#!/usr/bin/env node
process.on("SIGTERM", () => {});
setInterval(() => {}, 1000);
`);
    await chmod(binary, 0o755);
    const startedAt = Date.now();
    const result = await runPlugin(
      pluginRequest("browserjr.session", {
        commands: ["get title"],
        binary,
        timeoutMs: 25,
      }),
    );
    assert.ok(Date.now() - startedAt < 2_500);
    assert.equal(result.code, 0);
    const response = JSON.parse(result.stdout);
    assert.equal(response.success, false);
    assert.match(response.error, /timed out/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("relay keeps one browser.jr JSON session alive across plugin requests", async () => {
  const directory = await mkdtemp(join(tmpdir(), "browser-jr-plugin-relay-test-"));
  const binary = await fakeBrowserJr(directory);
  const token = "test-token";
  const relay = spawn(process.execPath, [CLI, "serve", "--binary", binary, "--token", token], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  try {
    const ready = await new Promise((resolvePromise, rejectPromise) => {
      let pending = "";
      relay.stdout.setEncoding("utf8");
      relay.stdout.on("data", (chunk) => {
        pending += chunk;
        const boundary = pending.indexOf("\n");
        if (boundary >= 0) resolvePromise(JSON.parse(pending.slice(0, boundary)));
      });
      relay.once("error", rejectPromise);
    });
    assert.equal(ready.success, true);
    const request = (command) =>
      pluginRequest("browserjr.command", {
        host: ready.result.host,
        port: ready.result.port,
        token,
        command,
      });
    const first = JSON.parse((await runPlugin(request("open https://example.com"))).stdout);
    const second = JSON.parse((await runPlugin(request("get title"))).stdout);
    assert.equal(first.result.sequence, 1);
    assert.equal(first.result.output, "ran:open https://example.com");
    assert.equal(second.result.sequence, 2);
    assert.equal(second.result.output, "ran:get title");
  } finally {
    relay.kill("SIGTERM");
    await new Promise((resolvePromise) => relay.once("close", resolvePromise));
    await rm(directory, { recursive: true, force: true });
  }
});


test("relay shutdown destroys idle client sockets", async () => {
  const directory = await mkdtemp(join(tmpdir(), "browser-jr-plugin-shutdown-test-"));
  const binary = await fakeBrowserJr(directory);
  const relay = spawn(process.execPath, [CLI, "serve", "--binary", binary], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  let socket;
  try {
    const ready = await new Promise((resolvePromise, rejectPromise) => {
      let pending = "";
      relay.stdout.setEncoding("utf8");
      relay.stdout.on("data", (chunk) => {
        pending += chunk;
        const boundary = pending.indexOf("\n");
        if (boundary >= 0) resolvePromise(JSON.parse(pending.slice(0, boundary)));
      });
      relay.once("error", rejectPromise);
    });
    socket = createConnection({ host: ready.result.host, port: ready.result.port });
    await new Promise((resolvePromise, rejectPromise) => {
      socket.once("connect", resolvePromise);
      socket.once("error", rejectPromise);
    });
    relay.kill("SIGTERM");
    await Promise.race([
      new Promise((resolvePromise) => relay.once("close", resolvePromise)),
      new Promise((_, rejectPromise) =>
        setTimeout(() => rejectPromise(new Error("relay did not stop")), 2_000),
      ),
    ]);
  } finally {
    socket?.destroy();
    if (relay.exitCode === null) relay.kill("SIGKILL");
    await rm(directory, { recursive: true, force: true });
  }
});
