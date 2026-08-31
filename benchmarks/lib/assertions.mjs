export function assertIncludes(value, expected, context) {
  if (!String(value).includes(expected)) {
    throw new Error(`${context} did not include ${JSON.stringify(expected)}`);
  }
}

export function assertEqual(actual, expected, context) {
  if (actual !== expected) {
    throw new Error(`${context} expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
  }
}
