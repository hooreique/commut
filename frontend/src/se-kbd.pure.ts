export type SeJamoName =
  | '_ㄱ'
  | '_ㄲ'
  | '_ㄴ'
  | '_ㄶ'
  | '_ㄷ'
  | '_ㄺ'
  | '_ㄻ'
  | '_ㄹ'
  | '_ㅁ'
  | '_ㅂ'
  | '_ㅄ'
  | '_ㅅ'
  | '_ㅆ'
  | '_ㅇ'
  | '_ㅈ'
  | '_ㅊ'
  | '_ㅋ'
  | '_ㅌ'
  | '_ㅍ'
  | '_ㅎ'
  | 'ㄱ'
  | 'ㄲ'
  | 'ㄴ'
  | 'ㄷ'
  | 'ㄸ'
  | 'ㄹ'
  | 'ㅁ'
  | 'ㅂ'
  | 'ㅃ'
  | 'ㅅ'
  | 'ㅆ'
  | 'ㅇ'
  | 'ㅈ'
  | 'ㅉ'
  | 'ㅊ'
  | 'ㅋ'
  | 'ㅌ'
  | 'ㅍ'
  | 'ㅎ'
  | 'ㅏ'
  | 'ㅐ'
  | 'ㅑ'
  | 'ㅒ'
  | 'ㅓ'
  | 'ㅔ'
  | 'ㅕ'
  | 'ㅖ'
  | 'ㅗ'
  | 'ㅘ'
  | 'ㅙ'
  | 'ㅛ'
  | 'ㅜ'
  | 'ㅝ'
  | 'ㅠ'
  | 'ㅡ'
  | 'ㅢ'
  | 'ㅣ';

export type SeKey = Readonly<{
  lower: SeJamoName;
  upper?: SeJamoName;
  flushOnLongPress?: boolean;
}>;

export type SeKeyRow = Readonly<{
  label: string;
  keys: readonly SeKey[];
}>;

const key = (
  lower: SeJamoName,
  upper?: SeJamoName,
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

export const seKeyLabel = (jamo: SeJamoName): string =>
  jamo.startsWith('_') ? jamo.substring(1) : jamo;
