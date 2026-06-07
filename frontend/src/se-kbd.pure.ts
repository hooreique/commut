import type { jamo } from 'libse';

export type JamoName = keyof typeof jamo;

export type SeKey = Readonly<{
  lower: JamoName;
  upper?: JamoName;
  spaceOnLongPress?: boolean;
}>;

export type SeKeyRow = Readonly<{
  label: string;
  keys: readonly SeKey[];
}>;

const key = (
  lower: JamoName,
  upper?: JamoName,
  spaceOnLongPress = false,
): SeKey => ({ lower, upper, spaceOnLongPress });

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
      key('_ㄴ', '_ㄶ'),
      key('_ㅇ', '_ㅄ'),
      key('_ㄹ', '_ㅍ'),
      key('_ㅅ', '_ㅌ'),
      key('_ㅂ', '_ㄺ'),
      key('_ㄱ', '_ㄻ'),
      key('_ㅁ', '_ㅊ'),
      key('_ㄷ', '_ㅋ'),
    ]),
  },
  {
    label: '중성',
    keys: Object.freeze([
      key('ㅙ', 'ㅝ'),
      key('ㅜ', 'ㅠ'),
      key('ㅡ', 'ㅢ'),
      key('ㅗ', 'ㅛ'),
      key('ㅣ', 'ㅘ'),
      key('ㅏ', 'ㅑ'),
      key('ㅓ', 'ㅕ'),
      key('ㅐ', 'ㅒ'),
      key('ㅔ', 'ㅖ'),
    ]),
  },
  {
    label: '초성',
    keys: Object.freeze([
      key('ㄹ', 'ㅌ'),
      key('ㅅ', 'ㅆ'),
      key('ㄷ', 'ㄸ'),
      key('ㅁ', 'ㅍ'),
      key('ㄴ', 'ㅊ'),
      key('ㅇ', undefined, true),
      key('ㄱ', 'ㄲ'),
      key('ㅈ', 'ㅉ'),
      key('ㅂ', 'ㅃ'),
      key('ㅎ', 'ㅋ'),
    ]),
  },
]);
