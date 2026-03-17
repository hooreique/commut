/**
 * Wraps a `fetch` call in a new W3C trace context by injecting a `traceparent` header.
 *
 * @param fetch - The original fetch function.
 * @returns A wrapped fetch that adds a W3C-compliant `traceparent` header.
 * @see https://www.w3.org/TR/trace-context-1/
 */
export const withTrace = (fetch: Fetch): Fetch => {
  const traceId = crypto.getRandomValues(new Uint8Array(16)).toHex();

  return (...args: Parameters<Fetch>) => fetch(args[0], {
    ...(args[1] ?? {}),
    headers: {
      traceparent: `00-${traceId
        }-${crypto.getRandomValues(new Uint8Array(8)).toHex()
        }-01`,
      ...(args[1]?.headers ?? {}),
    },
  });
};
