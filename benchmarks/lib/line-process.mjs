import { spawn } from "node:child_process";

export class LineProcess {
  #child;
  #closedError;
  #exitPromise;
  #lines = [];
  #pending = "";
  #stderr = "";
  #waiters = [];

  constructor(command, args, options = {}) {
    this.#child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env ?? process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.#child.stdout.setEncoding("utf8");
    this.#child.stderr.setEncoding("utf8");
    this.#child.stdout.on("data", (chunk) => this.#accept(chunk));
    this.#child.stderr.on("data", (chunk) => {
      this.#stderr += chunk;
    });
    this.#child.on("error", (error) => this.#close(error));
    this.#exitPromise = new Promise((resolvePromise) => {
      this.#child.on("close", (code, signal) => {
        this.#close(new Error(`process closed with code ${code} and signal ${signal}`));
        resolvePromise();
      });
    });
  }

  get stderr() {
    return this.#stderr;
  }

  #accept(chunk) {
    this.#pending += chunk;
    let boundary = this.#pending.indexOf("\n");
    while (boundary >= 0) {
      this.#lines.push(this.#pending.slice(0, boundary));
      this.#pending = this.#pending.slice(boundary + 1);
      boundary = this.#pending.indexOf("\n");
    }
    this.#drain();
  }

  #close(error) {
    if (this.#closedError) return;
    this.#closedError = error;
    for (const waiter of this.#waiters.splice(0)) {
      clearTimeout(waiter.timer);
      waiter.reject(error);
    }
  }

  #drain() {
    while (this.#lines.length > 0 && this.#waiters.length > 0) {
      const line = this.#lines.shift();
      const waiter = this.#waiters.shift();
      clearTimeout(waiter.timer);
      waiter.resolve(line);
    }
  }

  readLine(timeoutMs = 10_000) {
    if (this.#lines.length > 0) return Promise.resolve(this.#lines.shift());
    if (this.#closedError) return Promise.reject(this.#closedError);
    return new Promise((resolvePromise, rejectPromise) => {
      const waiter = {
        resolve: resolvePromise,
        reject: rejectPromise,
        timer: setTimeout(() => {
          const index = this.#waiters.indexOf(waiter);
          if (index >= 0) this.#waiters.splice(index, 1);
          rejectPromise(new Error(`timed out waiting for process output; stderr: ${this.#stderr}`));
        }, timeoutMs),
      };
      this.#waiters.push(waiter);
    });
  }

  async sendUntil(command, predicate, timeoutMs = 10_000) {
    this.#child.stdin.write(`${command}\n`);
    const lines = [];
    while (true) {
      const line = await this.readLine(timeoutMs);
      lines.push(line);
      if (predicate(line, lines)) return lines;
    }
  }

  async close() {
    if (this.#child.exitCode !== null) return;
    this.#child.stdin.write("exit\n");
    try {
      await this.readLine(2_000);
    } finally {
      if (this.#child.exitCode === null) this.#child.kill("SIGTERM");
    }
    await this.#exitPromise;
  }
}
