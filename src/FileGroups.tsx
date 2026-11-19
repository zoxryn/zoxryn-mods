import {
  FileCode2,
  Folder,
  Layers3,
  Shirt,
  Image,
  Settings2,
  Volume2,
  Activity,
} from 'lucide-react';
import { useMemo, useState } from 'react';
const groups = [
  { name: 'Scripts & loaders', icon: FileCode2, extensions: ['asi', 'dll', 'cs', 'vb'] },
  { name: 'Suits & ped models', icon: Shirt, extensions: ['ydd', 'yft', 'ymt'] },
  { name: 'Textures', icon: Image, extensions: ['ytd', 'png', 'jpg', 'jpeg', 'webp'] },
  { name: 'Sounds & audio', icon: Volume2, extensions: ['wav', 'mp3', 'ogg', 'awc'] },
  { name: 'Animations', icon: Activity, extensions: ['ycd', 'ifp'] },
  {
    name: 'Configuration & metadata',
    icon: Settings2,
    extensions: ['ini', 'cfg', 'json', 'xml', 'dat', 'meta'],
  },
  { name: 'Archives & other associated files', icon: Layers3, extensions: [] },
];
function Tree({ files, depth = 0 }: { files: string[]; depth?: number }) {
  const [limit, setLimit] = useState(40),
    [opened, setOpened] = useState<string[]>([]);
  const branches = useMemo(() => {
    const result = new Map<string, string[]>();
    for (const file of files) {
      const name = file.split('/')[depth] ?? file;
      let branch = result.get(name);
      if (!branch) {
        branch = [];
        result.set(name, branch);
      }
      branch.push(file);
    }
    return [...result];
  }, [files, depth]);
  return (
    <ul>
      {branches.slice(0, limit).map(([name, paths]) =>
        paths.some((p) => p.split('/').length > depth + 1) ? (
          <li key={name}>
            <details
              onToggle={(e) => {
                if (e.currentTarget.open)
                  setOpened((old) => (old.includes(name) ? old : [...old, name]));
              }}
            >
              <summary>
                <Folder size={12} />
                {name}
                <span>{paths.length}</span>
              </summary>
              {opened.includes(name) && <Tree files={paths} depth={depth + 1} />}
            </details>
          </li>
        ) : (
          <li key={name}>
            <FileCode2 size={12} />
            <code title={paths[0]} aria-label={paths[0]}>
              {name}
            </code>
          </li>
        ),
      )}
      {limit < branches.length && (
        <li>
          <button className="text-button" onClick={() => setLimit((n) => n + 40)}>
            Show more ({branches.length - limit} remaining)
          </button>
        </li>
      )}
    </ul>
  );
}
export default function FileGroups({ files }: { files: string[] }) {
  const grouped = groups.map((g) => ({ ...g, files: [] as string[] }));
  for (const file of [...new Set(files)]) {
    const ext = file.split('.').at(-1)?.toLowerCase() ?? '';
    (grouped.find((g) => g.extensions.includes(ext)) ?? grouped[grouped.length - 1]).files.push(
      file,
    );
  }
  return (
    <div className="file-groups">
      {grouped
        .filter((g) => g.files.length)
        .map((g) => (
          <FileGroup key={g.name} group={g} />
        ))}
    </div>
  );
}
function FileGroup({ group }: { group: (typeof groups)[number] & { files: string[] } }) {
  const [open, setOpen] = useState(false);
  return (
    <details onToggle={(e) => setOpen(e.currentTarget.open)}>
      <summary>
        <group.icon size={16} />
        {group.name}
        <span>{group.files.length}</span>
      </summary>
      {open && <Tree files={group.files} />}
    </details>
  );
}
