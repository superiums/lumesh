#!/usr/bin/env python3
"""Batch convert fish-style completion CSV entries to lumesh format.

Handles ALL known fish function patterns generically:
  - *no_subcommand / *needs_command / *no_command / *needs_subcommand → empty conds
  - *using_command X / *using_subcommand X / *command X / *subcommand X / *commands X → conds = X
  - *has_operation → empty + @n
  - *use_package / *use_port → @F
  - *has_no_argument → empty conds
  - __fish_seen_subcommand_from X → conds = X
  - __fish_seen_argument → extract short/long
  - __fish_contains_opt / __fish_not_contain_opt → conds with opt names (+ @n for not)
  - __fish_is_first_arg / __fish_is_first_token → empty conds
  - __fish_prev_arg_in X → conds = X
  - __fish_is_nth_token → empty conds
  - __fish_git_needs_command → empty + @n
  - And all other __fish_ functions fallback to empty conds (+ @E for desc exec scripts)
"""

import os
import re
import sys

NEW_DIR = "docs/data/completion_new/completions"
LUMESH_DIR = "docs/data/lumesh/completions"

LUMESH_HEADER = "cmd,conds,short,long,argument,dirs,pri,desc\n"

FISH_FUNC = re.compile(r'__fish_[a-zA-Z0-9_]+')

# === Generic pattern detection ===

def matches(pattern, name):
    """Check if name matches a generic fish function pattern.
    pattern is a regex like r'no_subcommand' or r'(?:using_command|command)\s+'."""
    return bool(re.search(pattern, name))


def parse_condition(cond_str):
    """Convert any fish condition to (conds_str, extra_dirs_list)."""
    c = cond_str.strip()
    extra_dirs = []

    if not c:
        return ' ', extra_dirs

    # === Detect `not ` prefix ===
    not_prefix = c.startswith('not ')
    if not_prefix:
        body = c[4:].strip()
    else:
        body = c

    # === Compound conditions: "X; and Y" ===
    # e.g. "__fish_meson_using_command wrap; and __fish_seen_subcommand_from update"
    # Means: "wrap subcommand is active AND update is seen"
    parts_compound = re.split(r';\s*and\s+', body)
    if len(parts_compound) > 1:
        # Collect all subcommand names from all parts
        all_subcmds = []
        compound_not = not_prefix
        for part in parts_compound:
            part = part.strip()
            pbody = part
            pnot = False
            if pbody.startswith('not '):
                pnot = True
                pbody = pbody[4:].strip()

            # Try to extract subcommand from each part
            m = re.match(r'__fish_seen_subcommand_from\s+(.*)', pbody)
            if m:
                all_subcmds.extend(m.group(1).split())
                continue

            m = re.match(r'__fish_[a-zA-Z0-9_]+_(?:using_subcommand|using_command|command|subcommand|commands)\s+(.*)', pbody)
            if m:
                all_subcmds.extend(m.group(1).split())
                continue

            m = re.match(r'__fish_[a-zA-Z0-9_]+_has_operation', pbody)
            if m:
                # has_operation → the command has an operation → we list nothing special
                # This is the inverse of no_subcommand
                compound_not = True  # effectively adds @n
                continue

            # Plain subcommand word
            for word in pbody.split():
                if not word.startswith('__fish_') and word not in ('and', 'not', '-s', '-l', 'test'):
                    all_subcmds.append(word)

        if compound_not:
            extra_dirs.append('@n')
        if all_subcmds:
            return ' '.join(all_subcmds), extra_dirs
        else:
            return ' ', extra_dirs

    # === Simple conditions ===
    c = body

    # --- Empty conds patterns ---
    # *no_subcommand / *needs_command / *no_command / *needs_subcommand
    # *has_no_argument / *no_arguments
    if matches(r'(?:no_subcommand|needs_command|no_command|needs_subcommand|no_subcmd|no_args|has_no_argument|no_arguments|python_no_arg|is_first_arg|is_first_token|should_complete_switches|is_switch|is_nth_token\s*\d*)', c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # *has_operation → empty + @n (the command has any operation selected)
    if matches(r'has_operation', c):
        return ' ', ['@n']

    # --- Extract subcommand name patterns ---
    # __fish_seen_subcommand_from X Y Z
    m = re.match(r'__fish_seen_subcommand_from\s+(.*)', c)
    if m:
        cmds = m.group(1).strip()
        if not_prefix:
            extra_dirs.append('@n')
        return cmds, extra_dirs

    # __fish_seen_argument -s X -l Y
    m = re.match(r'__fish_seen_argument\s*(-s\s+\S+)?\s*(-l\s+\S+)?\s*(-o\s+\S+)?', c)
    if m:
        names = []
        if m.group(1): names.append('-' + m.group(1).replace('-s ', ''))
        if m.group(2): names.append(m.group(2).replace('-l ', ''))
        if m.group(3): names.append(m.group(3).replace('-o ', ''))
        return ' '.join(names), extra_dirs

    # *using_command X / *using_subcommand X / *command X / *subcommand X / *commands X
    # e.g. __fish_meson_using_command wrap, __fish_fossil_command add, __fish_sops_commands exec-env
    m = re.match(r'__fish_[a-zA-Z0-9_]+_(?:using_command|using_subcommand|command|subcommand|commands)\s+(.*)', c)
    if m:
        cmds = m.group(1).strip()
        if not_prefix:
            extra_dirs.append('@n')
        return cmds, extra_dirs

    # --- Dirs directive patterns ---
    # *use_package / *use_package_installed / *use_port → @F (file completion for packages)
    if matches(r'use_package|use_port', c):
        extra_dirs.append('@F')
        return ' ', extra_dirs

    # --- Option patterns ---
    # __fish_contains_opt
    m = re.match(r'__fish_contains_opt\s+(.*)', c)
    if m:
        opts = _parse_opt_args(m.group(1).strip())
        if not_prefix:
            extra_dirs.append('@n')
        return ' '.join(opts), extra_dirs

    # __fish_not_contain_opt
    m = re.match(r'__fish_not_contain_opt\s+(.*)', c)
    if m:
        opts = _parse_opt_args(m.group(1).strip())
        extra_dirs.append('@n')
        return ' '.join(opts), extra_dirs

    # __fish_prev_arg_in X Y
    m = re.match(r'__fish_prev_arg_in\s+(.*)', c)
    if m:
        args = m.group(1).strip()
        if not_prefix:
            extra_dirs.append('@n')
        return args, extra_dirs

    # __fish_git_needs_command → empty + @n
    if matches(r'git_needs_command', c):
        return ' ', ['@n']

    # --- Generic fallback for any remaining __fish_XXX → empty conds ---
    if FISH_FUNC.search(c):
        return ' ', extra_dirs

    # --- Plain condition (no fish function) ---
    if not_prefix:
        extra_dirs.append('@n')
    return c, extra_dirs


def _parse_opt_args(s):
    """Parse -s / -l / plain option names from a string."""
    parts = s.split()
    names = []
    i = 0
    while i < len(parts):
        if parts[i] == '-s' and i + 1 < len(parts):
            names.append('-' + parts[i + 1])
            i += 2
        elif parts[i] == '-l' and i + 1 < len(parts):
            names.append(parts[i + 1])
            i += 2
        else:
            names.append(parts[i])
            i += 1
    return names


def parse_csv_line(line):
    """Parse a CSV line with proper quote handling."""
    parts = []
    current = ''
    in_quotes = False
    for ch in line.strip() + ',':
        if ch == '"':
            in_quotes = not in_quotes
        elif ch == ',' and not in_quotes:
            parts.append(current)
            current = ''
        else:
            current += ch
    return parts


def get_signature(parts):
    """Normalized signature for dedup: cmd|conds|short|long|argument"""
    return f"{parts[0].strip()}|{parts[1].strip()}|{parts[2].strip()}|{parts[3].strip()}|{parts[4].strip()}"


def convert_line(line, cmd_override=None):
    """Convert a single CSV line from completion_new format to lumesh format."""
    parts = parse_csv_line(line)
    if len(parts) < 8:
        return None

    cmd = parts[0].strip()
    conds = parts[1].strip()
    short = parts[2].strip()
    long_opt = parts[3].strip()
    argument = parts[4].strip()
    dirs = parts[5].strip()
    pri = parts[6].strip()
    desc = parts[7].strip()

    # Fix command name
    cmd = cmd.replace('$progname', 'pacman').replace('$bat', 'bat')
    if cmd_override:
        cmd = cmd_override

    # Handle cmd column containing fish functions (e.g., "__fish_seen_subcommand_from run")
    if cmd.startswith('__fish_seen_subcommand_from '):
        m = re.match(r'__fish_seen_subcommand_from\s+(.*)', cmd)
        if m:
            conds = m.group(1).strip()
            cmd = cmd_override or cmd

    # Convert condition
    cond_str, extra_dirs = parse_condition(conds)

    # Build dirs list
    dir_list = [d for d in dirs.split() if d] if dirs else []
    dir_list.extend(extra_dirs)

    # Handle @E for exec scripts in desc
    desc_clean = desc.strip().strip('"')
    if FISH_FUNC.fullmatch(desc_clean) or desc_clean.startswith('command '):
        if '@E' not in dir_list:
            dir_list.append('@E')

    # Deduplicate directives
    dir_str = ' '.join(sorted(set(dir_list))) if dir_list else ' '

    # Escape desc
    if ',' in desc or '"' in desc:
        desc = '"' + desc.replace('"', "'") + '"'

    # Fill empty fields
    for field_idx in range(7):
        val = [cmd, cond_str, short, long_opt, argument, dir_str, pri][field_idx]
        if not val:
            if field_idx == 0:
                cmd = ' '
            elif field_idx == 1:
                cond_str = ' '
            elif field_idx == 2:
                short = ' '
            elif field_idx == 3:
                long_opt = ' '
            elif field_idx == 4:
                argument = ' '
            elif field_idx == 5:
                dir_str = ' '
            elif field_idx == 6:
                pri = ' '

    if not desc:
        desc = ' '

    return f"{cmd},{cond_str},{short},{long_opt},{argument},{dir_str},{pri},{desc}"


def main():
    new_files = set(os.listdir(NEW_DIR))
    lumesh_files = set(os.listdir(LUMESH_DIR))
    common = sorted(new_files & lumesh_files)

    total_added = 0
    for filename in common:
        new_path = os.path.join(NEW_DIR, filename)
        lumesh_path = os.path.join(LUMESH_DIR, filename)

        # Read existing lumesh entries
        with open(lumesh_path) as f:
            lumesh_content = f.read()
        lumesh_lines = lumesh_content.strip().split('\n')
        lumesh_data = lumesh_lines[1:] if len(lumesh_lines) > 1 else []

        # Build existing signature set
        existing_sigs = set()
        for ln in lumesh_data:
            if ln.strip():
                p = parse_csv_line(ln)
                if len(p) >= 8:
                    existing_sigs.add(get_signature(p))

        # Read completion_new entries
        with open(new_path) as f:
            new_content = f.read()
        new_lines = new_content.strip().split('\n')
        if len(new_lines) < 2:
            continue

        cmd_name = filename.replace('.csv', '')

        # Find max priority
        max_pri = 0
        for ln in lumesh_data:
            p = parse_csv_line(ln)
            if len(p) >= 7:
                try:
                    max_pri = max(max_pri, int(p[6].strip()))
                except ValueError:
                    pass

        # Convert and add missing entries
        converted_lines = []
        added_count = 0
        for nl in new_lines[1:]:
            if not nl.strip():
                continue
            converted = convert_line(nl, cmd_override=cmd_name)
            if converted:
                p_out = parse_csv_line(converted)
                if len(p_out) >= 8:
                    sig = get_signature(p_out)
                    if sig not in existing_sigs:
                        converted_lines.append(converted)
                        added_count += 1
                        existing_sigs.add(sig)

        if added_count > 0:
            # Re-prioritize
            out_lines = []
            for i, cl in enumerate(converted_lines):
                p = parse_csv_line(cl)
                if len(p) >= 7:
                    p[6] = str(max_pri + i + 1)
                    out_lines.append(','.join(p))
                else:
                    out_lines.append(cl)

            # Append to lumesh file
            append_str = '\n'.join(out_lines)
            if not lumesh_content.endswith('\n'):
                append_str = '\n' + append_str
            else:
                append_str = '\n' + append_str

            with open(lumesh_path, 'a') as f:
                f.write(append_str)

            print(f"  {filename}: added {added_count} entries")
            total_added += added_count

    print(f"\nTotal: {total_added} entries added across all files")


if __name__ == '__main__':
    main()
