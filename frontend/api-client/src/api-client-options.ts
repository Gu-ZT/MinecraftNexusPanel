export interface ApiClientOptions {
  baseUrl: string;
  getAccessToken?: () => string | undefined;
}
