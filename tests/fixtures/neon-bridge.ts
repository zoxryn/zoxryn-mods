import { type Page } from '@playwright/test';
import { bridge } from './organization-bridge';
export async function neonBridge(page: Page) {
  await bridge(page);
  await page.addInitScript(() => {
    const w = window as any,
      org = w.organizationState;
    const base = org.collections[0],
      member = org.available_members[0],
      suit = org.suits[0];
    const packages = [
      ['Spider-Man Mod', 'Superhero Mods'],
      ['Hulk Mod', 'Superhero Mods'],
      ['Batman Mod', 'Superhero Mods'],
      ['Iron Man Mod', 'Superhero Mods'],
      ['Superman Mod', 'Superhero Mods'],
      ['Harbor Callouts', 'Police & LSPDFR'],
      ['County Callouts', 'Police & LSPDFR'],
      ['Police Vehicle Pack', 'Vehicles'],
      ['Officer Wardrobe', 'Police & LSPDFR'],
      ['RAGE Plugin Hook (shared dependency)', 'Tools & Dependencies'],
      ['Neon Preset', 'Maps & Graphics'],
      ['Weapon Pack', 'Weapons'],
    ];
    org.collections = packages.map(([name, category], i) => {
      const files = [
        `scripts/${name}/Main.cs`,
        `scripts/${name}/Config/options.ini`,
        `scripts/${name}/Sounds/dispatch.wav`,
      ];
      const component = {
        ...member,
        reference: { kind: 'discovered', id: `component-${i}` },
        name,
        category,
        source: 'Manual',
        status: 'needs_review',
        location: files[0],
        files,
        mod_id: null,
        role: 'Inferred package boundary',
        control_reason:
          'This inferred package has not been adopted. Complete membership and shared files require review.',
      };
      org.available_members.push(component);
      const section = name.includes('RAGE Plugin Hook')
        ? 'RAGE Plugin Hook (shared dependency)'
        : name.includes('Callouts')
          ? 'Police Callout Packs'
          : name.includes('Vehicle')
            ? 'Police Vehicle Packs'
            : name.includes('Wardrobe')
              ? 'Police Uniforms & Peds'
              : null;
      return {
        ...base,
        id: `folder-${i}`,
        name,
        category,
        members: [component],
        unique_files: 3,
        status: 'needs_review',
        description: `${section ? section + ' · ' : ''}Synthetic package evidence · no game files`,
        relationship: 'Inferred from a descriptive parent folder; compatibility unverified',
      };
    });
    org.suits = packages
      .slice(0, 5)
      .map(([name], i) => ({
        ...suit,
        id: `skin-${i}`,
        name: `${name.replace(' Mod', '')} · Classic`,
        parent: name,
        files: [`scripts/${name}/Suits/Classic.ini`],
        source: 'Manual',
        status: 'needs_review',
        manageable: false,
        mod_id: null,
        control_reason:
          'Identified outfit metadata. External package ownership has not been reviewed; shared files cannot be independently toggled.',
      }));
    org.suits.push({
      ...suit,
      id: 'officer-skin',
      name: 'Officer Uniform',
      parent: 'Officer Wardrobe',
      category: 'Police Uniforms',
      status: 'needs_review',
      manageable: false,
      source: 'Manual',
      mod_id: null,
    });
    for (const outfit of org.suits) {
      const group = org.collections.find((g: any) => g.name === outfit.parent);
      if (group) {
        const component = {
          ...member,
          reference: { kind: 'suit', id: outfit.id },
          name: outfit.name,
          files: outfit.files,
          status: outfit.status,
          source: 'Manual',
          mod_id: null,
          role: 'Explicit external outfit declaration',
          control_reason: outfit.control_reason,
        };
        group.members.push(component);
        group.unique_files += outfit.files.length;
        org.available_members.push(component);
      }
    }
    org.unidentified = [
      {
        ...member,
        reference: { kind: 'discovered', id: 'unidentified-1' },
        name: 'mystery.fixture',
        files: ['unknown/mystery.fixture'],
        location: 'unknown/mystery.fixture',
        status: 'unknown',
        role: 'Unidentified file',
        mod_id: null,
        control_reason: 'No reliable parent package evidence.',
      },
    ];
    org.health.issues = [
      {
        severity: 'review',
        subject: 'Harbor Callouts',
        detail: 'Package compatibility has not been verified for this game build.',
      },
    ];
    w.fixtureGame = {
      path: 'C:/SyntheticGame',
      edition: 'Legacy',
      launcher: 'Synthetic fixture',
      version: null,
      warnings: [],
      prerequisites: [
        ['shv', 'Script Hook V'],
        ['asi', 'ASI Loader'],
        ['openiv', 'OpenIV'],
        ['shvdn', 'ScriptHookVDotNet'],
        ['rph', 'RAGE Plugin Hook'],
        ['lspdfr', 'LSPDFR'],
      ].map(([id, name]) => ({
        id,
        name,
        present: true,
        detail: 'Detected marker only · compatibility unverified',
        url: '',
      })),
    };
    w.fixtureBackups = [
      {
        id: 'qa-backup',
        mod_name: 'Solar Package · Synthetic',
        operation: 'install',
        created_at: 1700000000,
        files: 1,
        bytes: 1024,
        status: 'committed',
      },
    ];
  });
}
