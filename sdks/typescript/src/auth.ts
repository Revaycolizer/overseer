/** Authentication management for the Overseer client. */

import type { AuthResponse, UserProfile } from './types.js';

export class AuthManager {
  private _token?: string;
  private _user?: UserProfile;

  get token(): string | undefined {
    return this._token;
  }

  set token(value: string | undefined) {
    this._token = value;
  }

  get user(): UserProfile | undefined {
    return this._user;
  }

  /** Store auth response from login/register */
  setAuth(response: AuthResponse): void {
    this._token = response.token;
    this._user = response.user;
  }

  /** Clear stored auth state */
  clear(): void {
    this._token = undefined;
    this._user = undefined;
  }

  /** Whether a token is available */
  get isAuthenticated(): boolean {
    return this._token !== undefined;
  }

  /** Headers for authenticated requests */
  get authHeaders(): Record<string, string> {
    return this._token ? { Authorization: `Bearer ${this._token}` } : {};
  }
}
