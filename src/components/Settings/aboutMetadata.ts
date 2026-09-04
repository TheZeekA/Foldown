export const ABOUT_DEVELOPER = "Zeeka Limited";
export const ABOUT_EMAIL = "support@zeeka.nz";

export function formatVersion(version: string): string {
  const trimmedVersion = version.trim();
  return trimmedVersion ? `Version ${trimmedVersion}` : "Version unavailable";
}
