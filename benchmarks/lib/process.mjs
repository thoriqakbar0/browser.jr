import { spawn } from "node:child_process";

export function runCommand(command, args, options = {}) {
  const timeoutMs = options.timeoutMs ?? 30_000;
  const startedAt = performance.now();

  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env ?? process.env,
      stdio: [options.input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
    });
    const stdout = [];
    const stderr = [];
    let settled = false;
    let timeoutError = null;
    let forceKillTimer = null;

    const timer = setTimeout(() => {
      timeoutError = new Error(`${command} timed out after ${timeoutMs}ms`);
      child.kill("SIGTERM");
      forceKillTimer = setTimeout(() => child.kill("SIGKILL"), 1_000);
    }, timeoutMs);

    function finish(error, code = null, signal = null) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      clearTimeout(forceKillTimer);
      const result = {
        code,
        signal,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
        durationMs: performance.now() - startedAt,
      };
      if (error) rejectPromise(Object.assign(error, { result }));
      else resolvePromise(result);
    }

    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => finish(error));
    child.on("close", (code, signal) => finish(timeoutError, code, signal));

    if (options.input !== undefined) {
      child.stdin.end(options.input);
    }
  });
}

export async function runChecked(command, args, options = {}) {
  const result = await runCommand(command, args, options);
  if (result.code !== 0) {
    const detail = result.stderr.trim() || result.stdout.trim() || `exit ${result.code}`;
    throw new Error(`${command} ${args.join(" ")} failed: ${detail}`);
  }
  return result;
}
