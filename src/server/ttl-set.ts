/**
 * TTL 이 있는 Set 을 생성합니다.
 */
export const ttlSet = (opts?: {
  /**
   * Time to live (in millis)
   */
  readonly ttl?: number;

  /**
   * 요소가 추가된 후 Callback 됩니다.
   * @param str 추가된 요소
   * @param set self
   */
  readonly afterAdd?: (str: string, set: ReadonlySet<string>) => void;

  /**
   * 요소가 TTL 에 의해 제거된 후 Callback 됩니다.
   * @param str 제거된 요소
   * @param set self
   */
  readonly afterGone?: (str: string, set: ReadonlySet<string>) => void;

  /**
   * `delete` 메서드 호출에 의해 요소가 제거된 후 Callback 됩니다.
   * 이미 그런 요소가 없었다면 실행되지 않습니다.
   * @param str 제거된 요소
   * @param set self
   */
  readonly afterDelete?: (str: string, set: ReadonlySet<string>) => void;

  /**
   * `has` 메서드 호출에 의해 요소가 조회된 후 Callback 됩니다.
   * 조회에 성공했을 때 (그러한 요소가 Set 에 존재했을 때) 만 실행됩니다.
   * @param str 조회된 요소
   * @param set self
   */
  readonly afterHit?: (str: string, set: ReadonlySet<string>) => void;

  /**
   * `has` 메서드 호출에 의해 요소가 조회된 후 Callback 됩니다.
   * 조회에 실패했을 때 (그러한 요소가 Set 에 존재하지 않았을 때) 만 실행됩니다.
   * @param str 조회 실패한 요소
   * @param set self
   */
  readonly afterMiss?: (str: string, set: ReadonlySet<string>) => void;
}): {
  /**
   * 요소를 추가합니다.
   * @param str 추가할 요소
   */
  readonly add: (str: string) => void;

  /**
   * 요소가 있는지 조회합니다.
   * @param str 조회할 요소
   * @returns `true`: 있을 때, `false`: 없을 때
   */
  readonly has: (str: string) => boolean;

  /**
   * 요소를 지웁니다.
   * @param str 지울 요소
   */
  readonly delete: (str: string) => void;
} => {
  const set = new Set<string>();

  return {
    add: (str: string): void => {
      set.add(str);
      (opts?.afterAdd ?? (() => undefined))(str, set);
      setTimeout(() => {
        if (set.delete(str)) (opts?.afterGone ?? (() => undefined))(str, set);
      }, opts?.ttl ?? 5_000);
    },
    has: (str: string): boolean => {
      const hit = set.has(str);
      (
        (hit ? opts?.afterHit : opts?.afterMiss)
        ??
        (() => undefined)
      )(str, set);
      return hit;
    },
    delete: (str: string) => {
      if (set.delete(str)) (opts?.afterDelete ?? (() => undefined))(str, set);
    },
  };
};
