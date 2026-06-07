import type { jamo } from 'libse';

export type JamoName = keyof typeof jamo;

export type SeKey = Readonly<{
  lower: JamoName;
  upper?: JamoName;
  flushOnLongPress?: boolean;
}>;

export type SeKeyRow = Readonly<{
  label: string;
  keys: readonly SeKey[];
}>;

const key = (
  lower: JamoName,
  upper?: JamoName,
  flushOnLongPress = false,
): SeKey => ({ lower, upper, flushOnLongPress });

/**
 * Rows mirror frontend/se-layout.txt. The visible lower key is emitted on tap;
 * the upper key for the same column is emitted on long press.
 */
export const SE_KEY_ROWS: readonly SeKeyRow[] = Object.freeze([
  {
    label: '종성',
    keys: Object.freeze([
      key('_ㅆ', '_ㄲ'),
      key('_ㅎ', '_ㅈ'),
      key('_ㄴ', '_ㅍ'),
      key('_ㅇ', '_ㄷ'),
      key('_ㄹ', '_ㄺ'),
      key('_ㅅ', '_ㅊ'),
      key('_ㅂ', '_ㅄ'),
      key('_ㄱ', '_ㄶ'),
      key('_ㅁ', '_ㄻ'),
      key('_ㅌ', '_ㅋ'),
    ]),
  },
  {
    label: '중성',
    keys: Object.freeze([
      key('ㅝ', 'ㅘ'),
      key('ㅜ', 'ㅠ'),
      key('ㅓ', 'ㅔ'),
      key('ㅏ', 'ㅑ'),
      key('ㅣ', 'ㅞ'),
      key('ㅕ', 'ㅖ'),
      key('ㅡ', 'ㅙ'),
      key('ㅗ', 'ㅛ'),
      key('ㅐ', 'ㅒ'),
    ]),
  },
  {
    label: '초성',
    keys: Object.freeze([
      key('ㅎ', 'ㅋ'),
      key('ㅂ', 'ㅃ'),
      key('ㄷ', 'ㄸ'),
      key('ㄹ', 'ㅌ'),
      key('ㅇ', undefined, true),
      key('ㄱ', 'ㄲ'),
      key('ㄴ', 'ㅊ'),
      key('ㅁ', 'ㅍ'),
      key('ㅅ', 'ㅆ'),
      key('ㅈ', 'ㅉ'),
    ]),
  },
]);
