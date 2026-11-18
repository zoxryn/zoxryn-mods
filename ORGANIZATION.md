# Collections, suits and previews

Version 0.1.8 continues the existing application and signed updater. Collections organize references; they never acquire a second copy of file ownership.

## Reliable grouping

An imported archive is a verified package boundary. Its canonical mod record owns its operative files, and optional `organization` metadata names the collection and its suit components. Main files, configuration and supporting resources remain inspectable through that record. One package can declare multiple suits without making those suits individually writable.

Scanned OpenIV content is grouped by its discovered DLC pack or nearest standalone RPF boundary. The label explicitly describes container membership, not an identified single mod. RPFs inside a DLC remain in that DLC group. Unrelated loose files are not automatically merged based on names or an unverified guess about a popular mod.

Custom collections hold selected managed, external or suit references. A mod can appear in several collections while retaining one canonical record and ownership ledger. Duplicate references within one collection are rejected. Missing members remain visible for reconciliation. Removing a custom collection only removes its metadata and memberships. Automatic ownership/container relationships cannot be deleted. Member status aggregates to enabled, disabled, partial, unknown or needs review; missing/review members take precedence. File counts deduplicate aliases of the same physical target/container, including a package and its suit reference.

## Discovery and its limits

The six suit categories are Superhero Suits, Police Uniforms, Custom Peds, Character Replacements, Civilian Outfits, and Other / Unidentified. The view has search, category filtering, name/status/source sorting, grid/list layouts, paged results and a details panel. User-assigned names/categories never establish safe ownership or unlock management.

| Evidence | Result |
|---|---|
| Explicit suit declaration in a retained package | Named component, exact declared file references, known model when supplied |
| Loose supported `CPedModelInfo__InitDataList` / `InitDatas` / `Item` / `Name` metadata in scanned mods/AddonPeds paths | Declared model name; bindings and registration remain unverified |
| Loose `.ydd`/`.ymt`, ped/player-related `.yft`, or clothing/player-related `.ytd` | Character-resource candidate requiring review; no complete outfit is guessed |
| Readable OpenIV RPF7 TOC with relevant resource names | Individual resource candidates under their actual container; unrelated modifications can coexist |
| Encrypted/unsupported AddonPeds RPF | Unknown archive candidate with blocked controls |
| Original game RPF, nested/encrypted archive payload, arbitrary binary registry, or unknown package metadata | No unsupported decoding or registry editing |

Loose ped XML is at most 256 KiB, parsed without DTD/entities, with bounded nodes/depth/model names. General scanning limits still apply. RPF character clues are capped at 256 per archive and the suit view at 2,048 candidates, with warnings when limits are reached. Large model/RPF payloads are not hashed for organization. Discovery cannot prove that a resource differs from the original game or that a declared model is registered or usable. Rescan refreshes external evidence; stored older scans may need a rescan for new resource metadata.

## Safe individual enable/disable

Individual controls currently support only an installed **Zoxryn-owned package containing exactly one explicitly declared standalone suit** whose complete operative file set is one root ASI or one direct `scripts/*.cs`/`*.vb` source loader plus exact matching local `.ini`, `.cfg`, `.json`, `.xml` or `.dat` sidecars. Existing package import rules also apply. These are script-provided outfits using existing resources; support does not imply arbitrary loose ped-model installation.

The complete declaration must agree with every file in the package and the original ownership ledger. Adopted/external packages, original-file replacements, incomplete ownership, multiple suit declarations, model assets and registration edits do not qualify. Hard links, missing dependencies, incomplete fresh scans, new same-stem loaders, or unlisted matching companions/resource directories block the plan.

Review the exact targets, dependency results and warnings. The user must confirm independent ownership/absence of unlisted registration and shared resources, author compatibility/game closure, and the exact file operation. Metadata alone cannot prove what arbitrary executable code references. If the author's instructions cannot establish independence, cancel.

Approval repeats the scope/file checks and uses the existing durable journal, verified content-addressed backups, ownership conflict checks, rollback and recovery engine. Disable removes only approved tracked files after backup; enable restores the exact verified bytes. Backups remain available. A new conflicting companion between review and approval cancels the operation. Collection edits, label changes and preview assignments never modify game files.

Shared RPF contents, AddonPeds editor/runtime registration, replacements, multi-suit bundles and externally scanned character assets are read-only here. Their controls explain why. No original RPF is edited, no mod scripts are run to infer relationships, and there is no in-game equip integration.

## Explicit package metadata

Add the optional `organization` object to the existing `zoxryn.json` schema. Paths are exact archive-relative paths after removal of one supported wrapper, use `/`, and must exist. Suit file lists are descriptive; they do not add new automated installation layouts or bypass the normal safety checks.

```json
{
  "name": "Example Script Outfit",
  "edition": "Legacy",
  "requirements": ["shv", "asi", "shvdn"],
  "organization": {
    "collection_name": "Example Outfits",
    "preview": "art/package.png",
    "suits": [{
      "key": "civilian",
      "name": "Example Civilian Outfit",
      "category": "Civilian Outfits",
      "model": "player_zero",
      "files": ["scripts/ExampleOutfit.cs", "scripts/ExampleOutfit.ini"],
      "standalone": true,
      "preview": "art/civilian.png"
    }]
  }
}
```

Preview files are non-operative artwork. Only explicitly declared artwork is imported; nearby images are not guessed. Invalid artwork yields an import warning and a placeholder. Invalid/missing file references, duplicate keys/references and unsafe paths are rejected. A package may declare up to 64 suits with up to 256 exact file references per suit.

## Preview storage and performance

Assign PNG, JPEG or WebP through the local picker; SVG and executable formats are unsupported. Inputs are limited to 8 MiB, 4,096 pixels per side and 4,194,304 pixels total, with decoder limits set before header inspection and decoding. Images become metadata-free PNG thumbnails up to 384 pixels per side, at most 640 KiB each, keyed by SHA-256 under the app data `thumbnails/` directory. Original image paths are not retained. Reparse/symlink paths and modified cache contents are rejected. No image/model code is executed.

The interface loads artwork near the viewport with at most three reads at a time and a 16-entry shared image cache. Cards use 24-item pages; expanded components and file lists reveal more on demand. Search is deferred. Metadata work runs on the native blocking-worker path under the existing operation gate, and updates wait for it to finish. There is no polling service. Cached disk thumbnails are retained conservatively; clearing an assignment removes its metadata reference, not shared cached content.

## Data preservation

SQLite schema 3 adds `collections`, `collection_members` and `visual_metadata`. Foreign keys prevent orphaned custom memberships. Existing mods, ownership, scan history, journals and settings are preserved. Before migrating an existing schema 0/1/2 database with user tables, a synced consistent copy is saved under `schema-migrations/`. The schema change runs in a transaction; newer unknown schemas are refused. Imported archives, blobs, backups and update trust/configuration keep their existing locations and identity. App-update checkpoints include the new tables. No database is reset.

Validation uses synthetic game folders and a separate updater QA product. Production builds exclude the QA loopback-feed exception and QA executable. Preparing local assets neither publishes nor installs an update.
