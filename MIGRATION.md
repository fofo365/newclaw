# OpenClaw to NewClaw Migration Guide

## Overview

NewClaw provides full compatibility with OpenClaw, including skill loading, workspace migration, and memory transfer. This guide shows you how to migrate your existing OpenClaw setup to NewClaw.

## Quick Start

### 1. Build Migration Tools

```bash
cd /root/newclaw
cargo build --release
```

### 2. Discover Your Skills

List all skills that will be migrated:

```bash
./target/release/migrate list-skills
```

This will scan `/root/.openclaw/workspace/skills/` and `/root/.openclaw/extensions/` for all available skills.

### 3. Migrate All Data

Perform a full migration (recommended):

```bash
./target/release/migrate migrate-all
```

This will migrate:
- Memory files (`MEMORY.md`, `memory/`)
- Skills (`workspace/skills/`, `extensions/`)
- Workspace files (`novels/`, `files/`, etc.)

### 4. Verify Migration

Check migrated files:

```bash
# Memory
ls -la /root/newclaw/MEMORY.md
ls -la /root/newclaw/memory/

# Workspace
ls -la /root/newclaw/workspace/
```

## Migration Details

### Memory Migration

OpenClaw's memory system is fully compatible:

| OpenClaw | NewClaw | Notes |
|----------|---------|-------|
| `workspace/MEMORY.md` | `MEMORY.md` | Direct copy |
| `workspace/memory/YYYY-MM-DD.md` | `memory/YYYY-MM-DD.md` | Preserved |
| `workspace/USER.md` | `USER.md` | Direct copy |
| `workspace/SOUL.md` | `SOUL.md` | Direct copy |
| `workspace/IDENTITY.md` | `IDENTITY.md` | Direct copy |

### Skill Compatibility

NewClaw can load OpenClaw skills without modification:

```rust
// Skills are auto-discovered from:
// - /root/.openclaw/workspace/skills/
// - /root/.openclaw/extensions/<name>/skills/
```

**Supported Skill Features:**
- ✅ SKILL.md frontmatter (name, description)
- ✅ Markdown documentation
- ✅ Scripts and tools
- ✅ Skill metadata

### Workspace Structure

Your workspace structure is preserved:

```
/root/newclaw/
├── workspace/
│   ├── novels/          # Migrated from OpenClaw
│   ├── files/           # Migrated from OpenClaw
│   ├── skills/          # Auto-discovered (not copied)
│   └── memory/          # Migrated (merged with memory/)
├── memory/              # Migrated from workspace/memory/
├── MEMORY.md            # Migrated from workspace/
├── USER.md              # Migrated from workspace/
├── SOUL.md              # Migrated from workspace/
└── IDENTITY.md          # Migrated from workspace/
```

## Advanced Usage

### Dry Run

Test migration without making changes:

```bash
./target/release/migrate migrate-all --dry-run
```

### Custom Paths

Migrate from/to custom locations:

```bash
./target/release/migrate migrate-all \
  --openclaw-path /custom/openclaw \
  --newclaw-path /custom/newclaw
```

### Memory-Only Migration

Migrate only memory files:

```bash
./target/release/migrate migrate-memory
```

## Skill Loading in NewClaw

After migration, skills are automatically available:

```rust
use newclaw::openclaw::{OpenClawMigrator, SkillManifest};

let migrator = OpenClawMigrator::new(
    PathBuf::from("/root/.openclaw"),
    PathBuf::from("/root/newclaw"),
);

// Load all skills
let result = migrator.migrate_skills()?;
for skill in &result.skills {
    println!("Loaded: {}", skill.name);
}
```

## Troubleshooting

### Skills Not Found

If skills aren't discovered:
1. Check that SKILL.md exists in skill directory
2. Verify frontmatter has `name:` and `description:`
3. Ensure directory path is correct

### Memory Files Missing

If memory files aren't migrated:
1. Check file permissions
2. Verify source paths exist
3. Check available disk space

### Permission Errors

If you encounter permission errors:
```bash
sudo chown -R $USER:$USER /root/newclaw
chmod -R 755 /root/newclaw
```

## Compatibility Matrix

| Feature | OpenClaw | NewClaw | Status |
|---------|----------|---------|--------|
| SKILL.md format | ✅ | ✅ | Fully compatible |
| Memory system | ✅ | ✅ | Fully compatible |
| Workspace | ✅ | ✅ | Fully compatible |
| Channels | ✅ | ✅ | Fully compatible |
| Plugins | ⚠️ | 🔌 | Different implementation |
| LLM Integration | ✅ | ✅ | Different providers |

## Next Steps

After migration:

1. **Test your skills**: Verify skills load correctly
2. **Update configurations**: Adjust paths if needed
3. **Run agents**: Start NewClaw agents with migrated data
4. **Monitor performance**: Compare with OpenClaw

## Rollback

If you need to rollback to OpenClaw:

```bash
# Your OpenClaw data is untouched
# Simply use OpenClaw commands as before
openclaw agent
```

Migration is **non-destructive** - OpenClaw data remains intact.

## Support

For issues or questions:
- Check NewClaw documentation
- Review migration logs
- Test with `--dry-run` first

---

**Note**: Migration is a one-time process. Subsequent NewClaw updates will not require re-migration.
