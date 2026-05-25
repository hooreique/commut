declare const naturalNumberFlag: unique symbol;

export type NaturalNumber = number & { [naturalNumberFlag]: never };

/** Returns whether a number is a positive safe integer. */
export const isNaturalNumber = (num: number): num is NaturalNumber => num > 0 && Number.isSafeInteger(num);

export type Dimensions = {
  readonly cols: NaturalNumber;
  readonly rows: NaturalNumber;
};
