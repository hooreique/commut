const te = new TextEncoder();
const blandSalt = te.encode('bland-salt');
const blandVect = te.encode('bland-vect');

export const enter = ({ passphrase, encpri }: {
  readonly passphrase: string;
  readonly encpri: string;
}): Promise<ArrayBuffer> => {
  if (!passphrase) {
    return Promise.reject({ message: 'password must not be empty' });
  }

  if (!encpri) {
    return Promise.reject({ message: 'encpri not found' });
  }

  return crypto.subtle.importKey('raw', te.encode(passphrase), 'PBKDF2', false, ['deriveKey'])
    .then(key => crypto.subtle.deriveKey({ name: 'PBKDF2', hash: 'SHA-256', salt: blandSalt, iterations: 300_000 }, key, { name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt']))
    .then(key => crypto.subtle.decrypt({ name: 'AES-GCM', iv: blandVect }, key, Uint8Array.fromBase64(encpri)))
    .catch(() => {
      throw { message: 'wrong password' };
    });
};
