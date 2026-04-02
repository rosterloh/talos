#!/bin/bash
set -e

# Global array for tracking packages that couldn't be installed.
failed_install=()

#######################################
# initialise_rosdep
# Initialises rosdep (if not already initialised) and updates its cache.
# Globals:
#   CONDA_PREFIX - expected to be set.
#######################################
initialise_rosdep() {
    local rosdep_sources="$CONDA_PREFIX/etc/ros/rosdep/sources.list.d/20-default.list"
    if [ ! -f "$rosdep_sources" ]; then
        echo "Initialising rosdep..."
        rosdep init
    fi
    echo "Updating rosdep..."
    rosdep update
}

#######################################
# build_ignore_filters
# Scans the current directory for any COLCON_IGNORE files and builds a composite
# string of "-not -path" filters.
# Outputs:
#   Echoes a filter string to be used in the find command.
#######################################
build_ignore_filters() {
    local ignore_dirs
    ignore_dirs=$(find . -type f -name COLCON_IGNORE -printf '%h\n' | sort -u)
    local filters=""
    for d in $ignore_dirs; do
        local d_clean
        d_clean=$(echo "$d" | sed 's:/*$::')
        filters="$filters -not -path \"$d_clean/*\""
    done
    echo "$filters"
}

#######################################
# find_package_files
# Locates package.xml files recursively in the current directory,
# excluding directories marked with COLCON_IGNORE and excluding any files
# under the directory specified by $CONDA_PREFIX.
# If $CONDA_PREFIX is inside the current directory, it is converted to a relative path.
# Arguments:
#   $1 - the -not -path filters from build_ignore_filters.
# Outputs:
#   A list of package.xml file paths.
#######################################
find_package_files() {
    local filters="$1"
    # Convert CONDA_PREFIX to a relative path if it's inside the current directory.
    local rel_conda_prefix="$CONDA_PREFIX"
    if [[ "$CONDA_PREFIX" == "$PWD"* ]]; then
        rel_conda_prefix="./${CONDA_PREFIX#$PWD/}"
    fi
    local extra_filters="-not -path \"${rel_conda_prefix}/*\""
    local find_cmd="find . -type f -name package.xml $filters $extra_filters"
    # Uncomment the next line to debug the find command:
    # echo "Running: $find_cmd"
    eval $find_cmd
}

#######################################
# extract_dependency_keys
# Extracts dependency keys from the given package.xml files.
# It searches for <depend>, <build_depend>, and <exec_depend> tags,
# ignoring any lines that are commented out.
# Arguments:
#   $1 - package_files (newline-separated list)
# Outputs:
#   A unique, sorted list of dependency keys.
#######################################
extract_dependency_keys() {
    local package_files="$1"
    echo "$package_files" \
      | xargs grep -hE '<(depend|build_depend|exec_depend)>[^<]+<\/(depend|build_depend|exec_depend)>' \
      | grep -vE '^\s*<!--' \
      | sed -E 's/<(depend|build_depend|exec_depend)>([^<]+)<\/(depend|build_depend|exec_depend)>/\2/' \
      | sort -u
}

#######################################
# extract_package_names
# Extracts package names from the given package.xml files.
# It searches for <name> tags, ignoring commented lines.
# Arguments:
#   $1 - package_files (newline-separated list)
# Outputs:
#   A unique, sorted list of package names.
#######################################
extract_package_names() {
    local package_files="$1"
    echo "$package_files" \
      | xargs grep -hE '<(name)>[^<]+<\/(name)>' \
      | grep -vE '^\s*<!--' \
      | sed -E 's/<(name)>([^<]+)<\/(name)>/\2/' \
      | sort -u
}

#######################################
# resolve_dependency_key
# Resolves a single dependency key using rosdep.
# Arguments:
#   $1 - The dependency key.
# Outputs:
#   Prints "key -> package" (or "(no mapping found)" if unresolved).
#######################################
resolve_dependency_key() {
    local key="$1"
    local output pkg

    # Capture all output (both stdout and stderr), and ignore commented lines.
    output=$(rosdep resolve "$key" 2>&1 | grep -v '^#')

    # If the output starts with an error, look for a pip mapping.
    if echo "$output" | grep -q "^ERROR:"; then
        # Use sed to start printing from the first "pip:" occurrence, then
        # grep the first line that starts with a dash (i.e. the package name)
        pkg=$(echo "$output" | sed -n '/pip:/,$p' | grep '^[[:space:]]*-[[:space:]]' | head -n 1 | sed 's/^[[:space:]]*-[[:space:]]*//')
    else
        # Otherwise, simply take the first line of the output.
        pkg=$(echo "$output" | head -n 1)
    fi

    if [ -n "$pkg" ]; then
        echo "$key -> $pkg"
    else
        echo "$key -> (no mapping found)"
    fi
}

#######################################
# resolve_dependency_keys_parallel
# Reads dependency keys from standard input and resolves them in parallel.
# Outputs:
#   Prints the mapping for each key.
#######################################
resolve_dependency_keys_parallel() {
    export -f resolve_dependency_key
    xargs -P "$(nproc)" -I {} bash -c 'resolve_dependency_key "$0"' {}
}

#######################################
# extract_valid_dependencies
# From the rosdep mapping output (lines like "key -> package"),
# extracts only the valid dependency package names (i.e. where a mapping exists),
# and only returns the first word of the resolved package.
# Arguments:
#   $1 - The mapping output (newline-separated).
# Outputs:
#   A unique, sorted list of valid dependency package names.
#######################################
extract_valid_dependencies() {
    local mappings="$1"
    echo "$mappings" | grep -v "\(no mapping found\)" | \
      awk -F ' -> ' '{split($2, a, " "); print a[1]}' | sort -u
}

#######################################
# add_packages_to_pixi
# Adds each valid dependency package using "pixi add" with --no-lockfile-update,
# skipping any package that fails to be added (echoing a message instead).
# Finally, updates the pixi lock file.
# Arguments:
#   $1 - Newline-separated list of valid dependency package names.
#######################################
add_packages_to_pixi() {
    local deps="$1"
    # Define colors.
    local YELLOW='\033[1;33m'
    local NC='\033[0m'  # No Color

    # Extract only the package names from the list (skip the header line).
    local installed
    installed=$(pixi list 2>/dev/null | tail -n +2 | awk '{print $1}')

    for pkg in $deps; do
        # Skip if the package is already installed.
        if echo "$installed" | grep -q "^$pkg\$"; then
            echo "$pkg is already added in pixi"
            continue
        fi
        # Check if the package exists in pixi using "pixi search".
        if ! pixi search "$pkg" >/dev/null 2>&1; then
            echo -e "${YELLOW}WARNING: Package '$pkg' is not valid in conda repository; skipping.${NC}"
            failed_install+=("$pkg")
            continue
        fi
        echo "Adding $pkg with pixi..."
        if pixi add "$pkg" --no-lockfile-update 2>/dev/null; then
            echo "$pkg added successfully"
        else
            echo -e "${YELLOW}WARNING: $pkg could not be added; skipping.${NC}"
            failed_install+=("$pkg")
        fi
    done

    echo "Updating pixi environment..."
    pixi update
}

#######################################
# print_unresolved_and_failed
# Prints unresolved dependency keys and packages that could not be installed.
# It takes the resolved mappings as input, extracts dependency keys marked as
# unresolved (i.e. with "(no mapping found)") and prints them. It also prints the
# packages that failed to be installed, one per line, in yellow.
#
# Globals:
#   failed_install - global array that stores packages which could not be installed.
#
# Arguments:
#   $1 - The resolved mappings (newline-separated list of dependency mappings).
#
# Outputs:
#   Prints the unresolved dependency keys and failed installations in yellow.
#######################################
print_unresolved_and_failed() {
    local resolved_mappings="$1"
    local YELLOW='\033[1;33m'
    local NC='\033[0m'
    local unresolved_keys

    # Capture keys that could not be resolved.
    unresolved_keys=$(echo "$resolved_mappings" | grep "\(no mapping found\)" | awk -F ' -> ' '{print $1}' | sort -u)

    if [ -n "$unresolved_keys" ]; then
        echo -e "${YELLOW}The following dependency keys could not be resolved:${NC}"
        while IFS= read -r dep; do
            echo -e "${YELLOW}$dep${NC}"
        done <<< "$unresolved_keys"
    fi

    echo
    if [ ${#failed_install[@]} -ne 0 ]; then
        echo -e "${YELLOW}The following packages could not be installed:${NC}"
        for pkg in "${failed_install[@]}"; do
            echo -e "${YELLOW}$pkg${NC}"
        done
    fi
}

main() {
    initialise_rosdep

    local ignore_filters
    ignore_filters=$(build_ignore_filters)

    local package_files
    package_files=$(find_package_files "$ignore_filters")
    # echo "Found package.xml files: (count: $(echo "$package_files" | wc -l))"
    # echo "$package_files"

    local package_names
    package_names=$(extract_package_names "$package_files")
    # echo "Extracted package names: (count: $(echo "$package_names" | wc -l))"
    # echo "$package_names"

    local dependency_keys
    dependency_keys=$(extract_dependency_keys "$package_files")
    # echo "Extracted dependency keys: (count: $(echo "$dependency_keys" | wc -l))"
    # echo "$dependency_keys"

    local filtered_dependency_keys
    filtered_dependency_keys=$(echo "$dependency_keys" | grep -vxFf <(echo "$package_names"))
    # echo "Filtered dependency keys (excluding package names): (count: $(echo "$filtered_dependency_keys" | wc -l))"
    # echo "$filtered_dependency_keys"

    echo "Resolving dependency keys in parallel..."
    local resolved_mappings
    resolved_mappings=$(echo "$filtered_dependency_keys" | resolve_dependency_keys_parallel)
    # echo "Resolved dependency mappings: (count: $(echo "$resolved_mappings" | wc -l))"
    # echo "$resolved_mappings"

    local valid_dependencies
    valid_dependencies=$(extract_valid_dependencies "$resolved_mappings")
    # echo "Valid dependency list: (count: $(echo "$valid_dependencies" | wc -l))"
    # echo "$valid_dependencies"

    # Add packages to pixi.
    echo "Processing pixi additions..."
    add_packages_to_pixi "$valid_dependencies"

    # At the end, print unresolved keys and failed installations in yellow, one per line.
    print_unresolved_and_failed "$resolved_mappings"
}

main