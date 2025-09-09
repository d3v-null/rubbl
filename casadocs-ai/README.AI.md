AI ingestion package for casacore (casa + tables)

What this is
- Doxygen-generated XML for the casacore `casa` and `tables` packages only, suitable for vectorizing and feeding to LLMs.

Where to look
- XML root: ./xml
  - index.xml: entrypoint listing all compounds (files, classes, namespaces, groups)
  - compound XML: one file per entity (e.g., Array_8h.xml, Table_8h.xml)

Recommended ingestion strategy
- Chunk by logical entity:
  - For each compound in index.xml, parse its brief and detailed descriptions, members, and relationships.
  - Keep fully-qualified names (e.g., casacore::Array) as primary keys.
- Retain hierarchy:
  - Namespace -> class/struct -> members.
  - Module grouping via Doxygen groups (e.g., group__Arrays__module, group__Tables__module).
- Store cross-refs:
  - Include base/derived relations, include graphs, and file paths.

Minimal fields per node
- fq_name, kind (class/struct/file/group), path, brief, details, members[], see_also[], includes[], inherits[], in_group[]

Notes
- These XML files were generated with EXTRACT_ALL=YES; undocumented members will appear with empty docs.
- Graphs are present (XML_PROGRAMLISTING=YES). Some external include references may be unresolved; safe to ignore.

Regenerate
```bash
# From repo root
DXY=./doxygen-ai.cfg
[ -f "$DXY" ] || echo "config missing: $DXY" >&2
command -v doxygen >/dev/null || echo "install doxygen" >&2
doxygen "$DXY"
```
