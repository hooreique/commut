declare global {
  namespace NodeJS {
    interface ProcessEnv {
      readonly HOME: string;
    }
  }
}

export { };
