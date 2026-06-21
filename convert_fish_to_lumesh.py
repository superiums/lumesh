#!/usr/bin/env python3
"""Batch convert fish-style completion CSV entries to lumesh format.

Reads completion_new CSV files, converts fish functions in conds/dirs columns,
and writes the missing entries to the corresponding lumesh CSV files.
"""

import os
import re
import csv
import io

NEW_DIR = "docs/data/completion_new/completions"
LUMESH_DIR = "docs/data/lumesh/completions"

LUMESH_HEADER = "cmd,conds,short,long,argument,dirs,pri,desc\n"

# Fish function patterns
FISH_NO_SUBCMD = re.compile(r'__fish_[a-zA-Z0-9_]*no_subcommand[a-zA-Z0-9_]*')
FISH_IS_FIRST_ARG = re.compile(r'__fish_is_first_(arg|token)')
FISH_NO_ARGUMENTS = re.compile(r'__fish_no_arguments')
FISH_PYTHON_NO_ARG = re.compile(r'__fish_python_no_arg')
FISH_PACMAN_HAS_OP = re.compile(r'__fish_pacman_has_operation')
FISH_SEEN_SUBCMD_FROM = re.compile(r'__fish_seen_subcommand_from\s+(.*)')
FISH_SEEN_ARGUMENT = re.compile(r'__fish_seen_argument\s+(-s\s+\S+)?\s*(-l\s+\S+)?')
FISH_NOT_CONTAIN_OPT = re.compile(r'__fish_not_contain_opt\s+(.*)')
FISH_CONTAINS_OPT = re.compile(r'__fish_contains_opt\s+(.*)')
FISH_PREV_ARG_IN = re.compile(r'__fish_prev_arg_in\s+(.*)')
FISH_IS_NTH_TOKEN = re.compile(r'__fish_is_nth_token\s+\d+')
FISH_IS_SWITCH = re.compile(r'__fish_is_switch')
FISH_SHOULD_COMPLETE_SWITCHES = re.compile(r'__fish_should_complete_switches')
FISH_SNAP_USE_FILE = re.compile(r'__fish_snap_use_file')
FISH_SNAP_USING_SUBCMD = re.compile(r'__fish_snap_using_subcommand\s+(.*)')
FISH_APT_USE_PACKAGE = re.compile(r'__fish_apt_use_package')
FISH_YUM_PACKAGE_OK = re.compile(r'__fish_yum_package_ok')
FISH_ENV_NOT_YET_VARS = re.compile(r'__fish_env_not_yet_vars')
FISH_JOURNALCTL_IS_FIELD = re.compile(r'__fish_journalctl_is_field')
FISH_APT_NO_SUBCMD = re.compile(r'__fish_apt_no_subcommand')

# Git-specific functions
FISH_GIT_NEEDS_COMMAND = re.compile(r'__fish_git_needs_command')
FISH_GIT_DASH_IN_TOKEN = re.compile(r'__fish_git_dash_in_token')
FISH_GIT_CONTAINS_OPT = re.compile(r'__fish_git_contains_opt\s+(.*)')
FISH_GIT_BRANCH_FOR_REMOTE = re.compile(r'__fish_git_branch_for_remote')
FISH_GIT_IS_REBASING = re.compile(r'__fish_git_is_rebasing')
FISH_GIT_POSSIBLE_COMMITHASH = re.compile(r'__fish_git_possible_commithash')
FISH_GIT_STASH_NOT_USING = re.compile(r'__fish_git_stash_not_using_subcommand')
FISH_GIT_STASH_IS_PUSH = re.compile(r'__fish_git_stash_is_push')
FISH_GIT_STASH_USING_CMD = re.compile(r'__fish_git_stash_using_command\s+(.*)')

# Generic fish function
FISH_FUNC = re.compile(r'__fish_[a-zA-Z0-9_]+')

# Also handle bat-specific functions
BAT_NO_EXCL = re.compile(r'-s -l')
BAT_CACHE_NO_EXCL = re.compile(r'__bat_cache_no_excl')
BAT_CACHE_SUBCMD = re.compile(r'__bat_cache_subcommand')


def parse_condition(cond_str, line_parts):
    """Convert fish condition string to lumesh conds + directives."""
    cond = cond_str.strip()
    extra_dirs = []

    if not cond:
        return ' ', extra_dirs

    # Check for compound conditions joined by "; and " or "&&"
    # e.g. "group; and __fish_seen_subcommand_from install"
    parts = re.split(r';\s*and\s+', cond)
    if len(parts) > 1:
        # Extract subcommand list from the compound
        subcmds = []
        has_not = False
        for part in parts:
            part = part.strip()
            m = FISH_SEEN_SUBCMD_FROM.search(part)
            if m:
                subcmds.extend(m.group(1).strip().split())
            m = FISH_SEEN_ARGUMENT.search(part)
            if m:
                short = m.group(1)
                long_opt = m.group(2)
                if short:
                    subcmds.append(short.replace('-s ', ''))
                if long_opt:
                    subcmds.append(long_opt.replace('-l ', ''))
            if 'not ' in part:
                has_not = True
            # Also check for other subcommand names
            for word in part.split():
                if word not in ('and', 'not', '__fish_seen_subcommand_from',
                                '__fish_seen_argument', '-s', '-l') \
                   and not word.startswith('-') and not word.startswith('__fish_'):
                    subcmds.append(word)

        if subcmds:
            if has_not:
                extra_dirs.append('@n')
            return ' '.join(subcmds), extra_dirs
        return cond, extra_dirs

    # Simple conditions
    # "not __fish_seen_subcommand_from mark"
    not_prefix = False
    c = cond
    if c.startswith('not '):
        not_prefix = True
        c = c[4:].strip()

    # __fish_*_no_subcommand → empty conds
    if FISH_NO_SUBCMD.match(c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # __fish_apt_no_subcommand
    if FISH_APT_NO_SUBCMD.match(c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # __fish_is_first_arg / __fish_is_first_token → empty conds
    if FISH_IS_FIRST_ARG.match(c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # __fish_no_arguments → empty conds
    if FISH_NO_ARGUMENTS.match(c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # __fish_python_no_arg → empty conds
    if FISH_PYTHON_NO_ARG.match(c):
        if not_prefix:
            extra_dirs.append('@n')
        return ' ', extra_dirs

    # __fish_pacman_has_operation → empty + @n (means "has any subcmds")
    if FISH_PACMAN_HAS_OP.match(c):
        return ' ', ['@n']

    # __fish_seen_subcommand_from X Y Z → conds = "X Y Z"
    m = FISH_SEEN_SUBCMD_FROM.match(c)
    if m:
        cmds = m.group(1).strip()
        if not_prefix:
            extra_dirs.append('@n')
        return cmds, extra_dirs

    # __fish_contains_opt → conds with option names
    m = FISH_CONTAINS_OPT.match(c)
    if m:
        opts = m.group(1).strip()
        # Extract arguments from -s/-l flags
        opt_parts = opts.split()
        names = []
        i = 0
        while i < len(opt_parts):
            if opt_parts[i] == '-s' and i + 1 < len(opt_parts):
                names.append('-' + opt_parts[i + 1])
                i += 2
            elif opt_parts[i] == '-l' and i + 1 < len(opt_parts):
                names.append(opt_parts[i + 1])
                i += 2
            else:
                names.append(opt_parts[i])
                i += 1
        if not_prefix:
            extra_dirs.append('@n')
        return ' '.join(names), extra_dirs

    # __fish_not_contain_opt → @n + option names
    m = FISH_NOT_CONTAIN_OPT.match(c)
    if m:
        opts = m.group(1).strip()
        opt_parts = opts.split()
        names = []
        i = 0
        while i < len(opt_parts):
            if opt_parts[i] == '-s' and i + 1 < len(opt_parts):
                names.append('-' + opt_parts[i + 1])
                i += 2
            elif opt_parts[i] == '-l' and i + 1 < len(opt_parts):
                names.append(opt_parts[i + 1])
                i += 2
            else:
                names.append(opt_parts[i])
                i += 1
        extra_dirs.append('@n')
        return ' '.join(names), extra_dirs

    # __fish_seen_argument -s d -l diff → conds with short/long
    m = FISH_SEEN_ARGUMENT.match(c)
    if m:
        short = m.group(1)
        long_opt = m.group(2)
        names = []
        if short:
            names.append('-' + short.replace('-s ', ''))
        if long_opt:
            names.append(long_opt.replace('-l ', ''))
        return ' '.join(names), extra_dirs

    # __fish_prev_arg_in X Y → conds with X Y
    m = FISH_PREV_ARG_IN.match(c)
    if m:
        args = m.group(1).strip()
        if not_prefix:
            extra_dirs.append('@n')
        return args, extra_dirs

    # __fish_is_nth_token N → empty
    if FISH_IS_NTH_TOKEN.match(c):
        return ' ', extra_dirs

    # __fish_should_complete_switches → empty
    if FISH_SHOULD_COMPLETE_SWITCHES.match(c):
        return ' ', extra_dirs

    # __fish_is_switch → empty
    if FISH_IS_SWITCH.match(c):
        return ' ', extra_dirs

    # __fish_snap_use_file → @F
    if FISH_SNAP_USE_FILE.match(c):
        return ' ', ['@F']

    # __fish_snap_using_subcommand X → conds X
    m = FISH_SNAP_USING_SUBCMD.match(c)
    if m:
        cmds = m.group(1).strip()
        return cmds, extra_dirs

    # __fish_apt_use_package → empty (the @E will be handled from desc)
    if FISH_APT_USE_PACKAGE.match(c):
        return ' ', extra_dirs

    # __fish_yum_package_ok → empty (the @E will be handled from desc)
    if FISH_YUM_PACKAGE_OK.match(c):
        return ' ', extra_dirs

    # __fish_env_not_yet_vars → empty conds
    if FISH_ENV_NOT_YET_VARS.match(c):
        return ' ', extra_dirs

    # __fish_journalctl_is_field → empty conds
    if FISH_JOURNALCTL_IS_FIELD.match(c):
        return ' ', extra_dirs

    # Git-specific
    if FISH_GIT_NEEDS_COMMAND.match(c):
        # needs_command → opposite of no_subcommand → empty + @n
        return ' ', ['@n']

    if FISH_GIT_DASH_IN_TOKEN.match(c):
        # has dash in current token → keep as empty
        return ' ', extra_dirs

    m = FISH_GIT_CONTAINS_OPT.match(c)
    if m:
        return m.group(1).strip(), extra_dirs

    if FISH_GIT_BRANCH_FOR_REMOTE.match(c):
        return ' ', extra_dirs

    if FISH_GIT_IS_REBASING.match(c):
        return ' ', extra_dirs

    if FISH_GIT_POSSIBLE_COMMITHASH.match(c):
        return ' ', extra_dirs

    if FISH_GIT_STASH_NOT_USING.match(c):
        return ' ', extra_dirs

    if FISH_GIT_STASH_IS_PUSH.match(c):
        return ' ', extra_dirs

    m = FISH_GIT_STASH_USING_CMD.match(c)
    if m:
        cmds = m.group(1).strip()
        return cmds, extra_dirs

    # bat-specific
    if BAT_NO_EXCL.match(c) or BAT_CACHE_NO_EXCL.match(c) or BAT_CACHE_SUBCMD.match(c):
        return ' ', extra_dirs

    # Any other __fish_ function → keep as empty guess
    if FISH_FUNC.search(c):
        return ' ', extra_dirs

    # Plain condition: word list
    if not_prefix:
        extra_dirs.append('@n')

    return c, extra_dirs


def convert_description(desc):
    """Check if description contains a fish function that should be an @E exec."""
    desc = desc.strip()
    # Check if description is a fish function call (dynamic completion)
    if FISH_FUNC.fullmatch(desc):
        return desc, True  # return as exec script and mark as @E
    return desc, False


def convert_line(line, cmd_override=None):
    """Convert a single CSV line from completion_new format to lumesh format."""
    # Parse the CSV line manually (simple comma split with quote handling)
    parts = []
    current = ''
    in_quotes = False
    for ch in line + ',':
        if ch == '"':
            in_quotes = not in_quotes
        elif ch == ',' and not in_quotes:
            parts.append(current)
            current = ''
        else:
            current += ch
    parts.append(current)

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

    # Fix command name: $progname → pacman, $bat → bat, etc.
    cmd = cmd.replace('$progname', 'pacman')
    cmd = cmd.replace('$bat', 'bat')
    if cmd_override:
        cmd = cmd_override

    # Convert condition
    cond_str, extra_dirs = parse_condition(conds, parts)

    # Convert description
    dir_list = [d for d in dirs.split() if d] if dirs else []
    if extra_dirs:
        dir_list.extend(extra_dirs)

    # If description is a fish function, it becomes exec
    is_exec = False
    if desc.startswith('__fish_') or desc.startswith('command '):
        # Check if there's an @E in dirs already
        if '@E' not in dir_list:
            dir_list.append('@E')
            is_exec = True

    # Deduplicate and sort directives
    dir_list = sorted(set(dir_list))
    dir_str = ' '.join(dir_list) if dir_list else ' '

    # Escape desc if needed
    if ',' in desc or '"' in desc:
        desc = '"' + desc.replace('"', "'") + '"'

    if cond_str is None:
        cond_str = ' '

    return f"{cmd},{cond_str},{short},{long_opt},{argument},{dir_str},{pri},{desc}"


def get_signature(line):
    """Get a normalized signature of a lumesh CSV line for dedup."""
    parts = line.split(',')
    if len(parts) < 8:
        return line
    # For dedup: match on cmd + conds + short + long + params
    key = f"{parts[0].strip()}|{parts[1].strip()}|{parts[2].strip()}|{parts[3].strip()}|{parts[4].strip()}"
    return key


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
        lumesh_lines_raw = lumesh_content.strip().split('\n')
        lumesh_header = lumesh_lines_raw[0] if lumesh_lines_raw else LUMESH_HEADER.strip()
        lumesh_data_lines = lumesh_lines_raw[1:] if len(lumesh_lines_raw) > 1 else []

        # Build signature set
        existing_signatures = set()
        for ln in lumesh_data_lines:
            if ln.strip():
                existing_signatures.add(get_signature(ln))

        # Read completion_new entries
        with open(new_path) as f:
            new_content = f.read()
        new_lines = new_content.strip().split('\n')
        if len(new_lines) < 2:
            continue

        cmd_name = filename.replace('.csv', '')

        # Convert and check for missing entries
        converted_lines = []
        added_count = 0
        for nl in new_lines[1:]:
            if not nl.strip():
                continue
            converted = convert_line(nl, cmd_override=cmd_name)
            if converted:
                sig = get_signature(converted)
                if sig not in existing_signatures:
                    converted_lines.append(converted)
                    added_count += 1

        if added_count > 0:
            # Append to lumesh file
            max_pri = 0
            for ln in lumesh_data_lines:
                parts = ln.split(',')
                if len(parts) >= 7:
                    try:
                        p = int(parts[6].strip())
                        max_pri = max(max_pri, p)
                    except ValueError:
                        pass

            # Re-prioritize new entries
            new_lines_final = []
            for i, cl in enumerate(converted_lines):
                parts = cl.split(',')
                parts[6] = str(max_pri + i + 1)
                new_lines_final.append(','.join(parts))

            # Append after header
            if lumesh_content.endswith('\n'):
                append_content = '\n' + '\n'.join(new_lines_final)
            else:
                append_content = '\n' + '\n'.join(new_lines_final)

            with open(lumesh_path, 'a') as f:
                f.write(append_content)

            print(f"  {filename}: added {added_count} entries")
            total_added += added_count

    print(f"\nTotal: {total_added} entries added across all files")


if __name__ == '__main__':
    main()
