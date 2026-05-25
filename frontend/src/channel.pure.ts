export type Channel<T> = {
  readonly emit: (thing: T) => void;
  readonly on: (listen: (thing: T) => void) => void;
};

export const channel = <T>(): Channel<T> => {
  let callback: (thing: T) => void = () => undefined;

  return {
    emit: thing => {
      callback(thing);
    },
    on: listen => {
      callback = listen;
    },
  };
};
