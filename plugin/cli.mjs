#!/usr/bin/env node
import { randomBytes } from "node:crypto";
import { spawn } from "node:child_process";
import { createConnection, createServer } from "node:net";
import { access } from "node:fs/promises";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const PROTOCOL = "agent-browser.plugin.v1";
const CAPABILITIES = Object.freeze(["command.run", "browserjr.session", "browserjr.command"]);
const PACKAGE_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DEFAULT_TIMEOUT_MS = 30_000;

function response(fields) {
  process.stdout.write(`${JSON.stringify({ protocol: PROTOCOL, ...fields })}\n`);
}

function failure(error) {
  return { success: false, error: error instanceof Error ? error.message : String(error) };
}

function requireObject(value, name) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }
  return value;
}

function requireString(value, name) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${name} must be a non-empty string`);
  }
  return value;
}

function requireCommand(value, name) {
  const command = requireString(value, name);
  if (command.length > 8_192) throw new TypeError(`${name} must not exceed 8192 characters`);
  if (command.includes("\n") || command.includes("\r")) {
    throw new TypeError(`${name} must not contain line breaks`);
  }
  if (command.trim() === "exit") throw new TypeError(`${name} must not be exit`);
  return command;
}

function optionalTimeout(value) {
  if (value === undefined) return DEFAULT_TIMEOUT_MS;
  if (!Number.isInteger(value) || value < 1 || value > 120_000) {
    throw new TypeError("timeoutMs must be an integer from 1 to 120000");
  }
  return value;
}

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function resolveBinary(explicit) {
  if (explicit) return explicit;
  if (process.env.BROWSER_JR_BIN) return process.env.BROWSER_JR_BIN;
  const filename = process.platform === "win32" ? "browser-jr.exe" : "browser-jr";
  const packaged = join(PACKAGE_ROOT, "target", "release", filename);
  return (await exists(packaged)) ? packaged : "browser-jr";
}

function collectProcess(command, args, options = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: process.env,
      stdio: [options.input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
    });
    const stdout = [];
    const stderr = [];
    let settled = false;
    let timedOut = false;
    let forceKillTimer = null;

    function forceKillSoon() {
      if (forceKillTimer) return;
      forceKillTimer = setTimeout(() => child.kill("SIGKILL"), 1_000);
    }

    function cleanup() {
      clearTimeout(timer);
      clearTimeout(forceKillTimer);
      process.removeListener("SIGINT", forwardSigint);
      process.removeListener("SIGTERM", forwardSigterm);
    }

    function settle(error, result) {
      if (settled) return;
      settled = true;
      cleanup();
      if (error) rejectPromise(error);
      else resolvePromise(result);
    }

    function forwardSignal(signal) {
      child.kill(signal);
      forceKillSoon();
    }

    function forwardSigint() {
      forwardSignal("SIGINT");
    }

    function forwardSigterm() {
      forwardSignal("SIGTERM");
    }

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
      forceKillSoon();
    }, options.timeoutMs ?? DEFAULT_TIMEOUT_MS);

    process.once("SIGINT", forwardSigint);
    process.once("SIGTERM", forwardSigterm);
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.once("error", (error) => settle(error));
    child.once("close", (code, signal) => {
      if (timedOut) {
        settle(new Error(`${command} timed out`));
        return;
      }
      settle(null, {
        code,
        signal,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
      });
    });
    if (options.input !== undefined) child.stdin.end(options.input);
  });
}

async function runBrowserJrSession(request) {
  const payload = requireObject(request, "request");
  const commands = payload.commands;
  if (!Array.isArray(commands) || commands.length < 1 || commands.length > 100) {
    throw new TypeError("request.commands must contain from 1 to 100 commands");
  }
  let totalBytes = 0;
  for (const value of commands) {
    const command = requireCommand(value, "each command");
    totalBytes += Buffer.byteLength(command);
  }
  if (totalBytes > 65_536) throw new TypeError("commands exceed the 65536-byte limit");
  if (payload.allowLoopback !== undefined && typeof payload.allowLoopback !== "boolean") {
    throw new TypeError("request.allowLoopback must be a boolean");
  }
  const binary = await resolveBinary(payload.binary);
  const args = [...(payload.allowLoopback ? ["--allow-loopback"] : []), "--json", "session"];
  const input = `${commands.join("\n")}\nexit\n`;
  const result = await collectProcess(binary, args, {
    input,
    timeoutMs: optionalTimeout(payload.timeoutMs),
  });
  if (result.code !== 0) {
    throw new Error(result.stderr.trim() || result.stdout.trim() || `browser.jr exited ${result.code}`);
  }
  const events = result.stdout
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  if (!events.some((event) => event.data?.event === "ready")) {
    throw new Error("browser.jr session did not emit a ready event");
  }
  if (!events.some((event) => event.data?.event === "closed")) {
    throw new Error("browser.jr session did not emit a closed event");
  }
  return { events, stderr: result.stderr };
}

function exchangeWithRelay(request) {
  const payload = requireObject(request, "request");
  const host = payload.host ?? "127.0.0.1";
  if (!new Set(["127.0.0.1", "::1", "localhost"]).has(host)) {
    throw new TypeError("request.host must be a loopback host");
  }
  const port = payload.port;
  const token = requireString(payload.token, "request.token");
  const command = requireCommand(payload.command, "request.command");
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new TypeError("request.port must be an integer from 1 to 65535");
  }
  const timeoutMs = optionalTimeout(payload.timeoutMs);
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = createConnection({ host, port });
    let pending = "";
    const timer = setTimeout(() => {
      socket.destroy();
      rejectPromise(new Error("browser.jr relay timed out"));
    }, timeoutMs);
    function finish(error, value) {
      clearTimeout(timer);
      socket.destroy();
      if (error) rejectPromise(error);
      else resolvePromise(value);
    }
    socket.setEncoding("utf8");
    socket.once("error", (error) => finish(error));
    socket.on("data", (chunk) => {
      pending += chunk;
      const boundary = pending.indexOf("\n");
      if (boundary < 0) return;
      try {
        const message = JSON.parse(pending.slice(0, boundary));
        if (!message.success) throw new Error(message.error ?? "browser.jr relay failed");
        finish(null, message.result);
      } catch (error) {
        finish(error);
      }
    });
    socket.once("connect", () => {
      socket.write(`${JSON.stringify({ token, command })}\n`);
    });
  });
}

function parseServeOptions(args) {
  const options = { host: "127.0.0.1", port: 0, allowLoopback: false, binary: null, token: null };
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    const value = args[index + 1];
    if (argument === "--allow-loopback") options.allowLoopback = true;
    else if (argument === "--host" && value) { options.host = value; index += 1; }
    else if (argument === "--port" && value) { options.port = Number.parseInt(value, 10); index += 1; }
    else if (argument === "--binary" && value) { options.binary = value; index += 1; }
    else if (argument === "--token" && value) { options.token = value; index += 1; }
    else throw new TypeError(`unknown or incomplete serve option: ${argument}`);
  }
  if (!Number.isInteger(options.port) || options.port < 0 || options.port > 65535) {
    throw new TypeError("--port must be an integer from 0 to 65535");
  }
  if (!new Set(["127.0.0.1", "::1", "localhost"]).has(options.host)) {
    throw new TypeError("--host must be a loopback host");
  }
  return options;
}

async function serve(args) {
  const options = parseServeOptions(args);
  const binary = await resolveBinary(options.binary);
  const childArgs = [...(options.allowLoopback ? ["--allow-loopback"] : []), "--json", "session"];
  const child = spawn(binary, childArgs, { stdio: ["pipe", "pipe", "pipe"] });
  const lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
  const iterator = lines[Symbol.asyncIterator]();
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  const ready = await Promise.race([
    iterator.next(),
    new Promise((_, rejectPromise) => child.once("error", rejectPromise)),
  ]);
  if (ready.done) throw new Error(`browser.jr closed before ready: ${stderr}`);
  const readyEnvelope = JSON.parse(ready.value);
  if (!readyEnvelope.success || readyEnvelope.data?.event !== "ready") {
    throw new Error("browser.jr did not emit a ready event");
  }

  const token = options.token ?? randomBytes(24).toString("hex");
  const sockets = new Set();
  let queue = Promise.resolve();
  const server = createServer((socket) => {
    sockets.add(socket);
    socket.once("close", () => sockets.delete(socket));
    socket.setEncoding("utf8");
    socket.setTimeout(DEFAULT_TIMEOUT_MS, () => socket.destroy());
    let pending = "";
    socket.on("data", (chunk) => {
      pending += chunk;
      const boundary = pending.indexOf("\n");
      const frame = boundary < 0 ? pending : pending.slice(0, boundary);
      if (Buffer.byteLength(frame) > 16_384) {
        socket.end(`${JSON.stringify(failure(new Error("relay request exceeds the 16384-byte limit")))}\n`);
        return;
      }
      if (boundary < 0) return;
      socket.removeAllListeners("data");
      queue = queue.then(async () => {
        try {
          const message = requireObject(JSON.parse(pending.slice(0, boundary)), "relay request");
          if (message.token !== token) throw new Error("browser.jr relay rejected the token");
          const command = requireCommand(message.command, "relay request.command");
          child.stdin.write(`${command}\n`);
          const next = await iterator.next();
          if (next.done) throw new Error(`browser.jr session closed: ${stderr}`);
          const envelope = JSON.parse(next.value);
          if (!envelope.success) throw new Error(envelope.error ?? "browser.jr command failed");
          socket.end(`${JSON.stringify({ success: true, result: envelope.data })}\n`);
        } catch (error) {
          socket.end(`${JSON.stringify(failure(error))}\n`);
        }
      });
    });
  });

  const close = () => {
    server.close();
    for (const socket of sockets) socket.destroy();
    if (child.exitCode === null) {
      child.stdin.end("exit\n");
      setTimeout(() => child.kill("SIGTERM"), 500).unref();
    }
  };
  process.once("SIGINT", close);
  process.once("SIGTERM", close);
  child.once("exit", () => server.close());
  await new Promise((resolvePromise, rejectPromise) => {
    server.once("error", rejectPromise);
    server.listen(options.port, options.host, resolvePromise);
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("relay did not receive a TCP port");
  response({ success: true, result: { event: "ready", host: options.host, port: address.port, token } });
}

async function pluginMain() {
  let input = "";
  for await (const chunk of process.stdin) input += chunk;
  const envelope = requireObject(JSON.parse(input), "plugin request");
  if (envelope.protocol !== PROTOCOL) throw new Error(`unsupported protocol: ${envelope.protocol ?? ""}`);
  if (envelope.type === "plugin.manifest") {
    response({
      success: true,
      manifest: { name: "browser-jr", capabilities: CAPABILITIES },
    });
    return;
  }
  if (envelope.type === "browserjr.session") {
    response({ success: true, result: await runBrowserJrSession(envelope.request) });
    return;
  }
  if (envelope.type === "browserjr.command") {
    response({ success: true, result: await exchangeWithRelay(envelope.request) });
    return;
  }
  throw new Error(`unsupported request type: ${envelope.type ?? ""}`);
}

const serving = process.argv[2] === "serve";
try {
  if (serving) await serve(process.argv.slice(3));
  else await pluginMain();
} catch (error) {
  response(failure(error));
  if (serving) process.exitCode = 1;
}
