function requireFiniteSamples(samples) {
  if (!Array.isArray(samples) || samples.length === 0) {
    throw new TypeError("samples must contain at least one number");
  }
  if (samples.some((sample) => !Number.isFinite(sample) || sample < 0)) {
    throw new TypeError("samples must contain finite non-negative numbers");
  }
}

function percentile(sorted, fraction) {
  const index = Math.max(0, Math.ceil(sorted.length * fraction) - 1);
  return sorted[index];
}

export function summarize(samples) {
  requireFiniteSamples(samples);
  const sorted = [...samples].sort((left, right) => left - right);
  const mean = sorted.reduce((total, sample) => total + sample, 0) / sorted.length;
  const variance =
    sorted.reduce((total, sample) => total + (sample - mean) ** 2, 0) / sorted.length;

  return {
    samples: sorted.length,
    minMs: sorted[0],
    medianMs: percentile(sorted, 0.5),
    p95Ms: percentile(sorted, 0.95),
    maxMs: sorted.at(-1),
    meanMs: mean,
    stddevMs: Math.sqrt(variance),
  };
}
