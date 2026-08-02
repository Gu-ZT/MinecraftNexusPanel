export interface ApiClientOptions {
  baseUrl: string;
  getAccessToken?: () => string | undefined;
  getCsrfToken?: () => string | undefined;
}
