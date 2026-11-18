// Phase 2 extension point. This release only opens publisher pages.
export interface DownloadProvider {
  id: string;
  name: string;
  description: string;
  homepage: string;
  capabilities: { browse: true; authorizedDownload: false };
  // An eventual implementation must obtain authorization and honor provider terms.
  resolveAuthorizedDownload?: (
    page: URL,
    authorization: unknown,
  ) => Promise<{ url: URL; expiresAt?: number }>;
}
export const providers: DownloadProvider[] = [
  {
    id: 'gta5mods',
    name: 'GTA5-Mods.com',
    homepage: 'https://www.gta5-mods.com/',
    description: 'Vehicles, scripts, graphics, and a whole new Los Santos.',
    capabilities: { browse: true, authorizedDownload: false },
  },
  {
    id: 'lcpdfr',
    name: 'LCPDFR.com',
    homepage: 'https://www.lcpdfr.com/',
    description: 'LSPDFR, police vehicles, callouts, and community creations.',
    capabilities: { browse: true, authorizedDownload: false },
  },
];
