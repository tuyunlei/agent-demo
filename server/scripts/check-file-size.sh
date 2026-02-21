#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

max_file_lines=300
max_fn_lines=50
violations=0

while IFS= read -r -d '' file; do
  if ! awk -v file="$file" -v max_file_lines="$max_file_lines" -v max_fn_lines="$max_fn_lines" '
    function is_effective(line) {
      return line !~ /^[[:space:]]*$/ && line !~ /^[[:space:]]*\/\//
    }
    function count_braces(line,   i, ch, in_str, in_chr, esc, c) {
      c = 0
      in_str = 0
      in_chr = 0
      esc = 0
      for (i = 1; i <= length(line); i++) {
        ch = substr(line, i, 1)
        if (esc) {
          esc = 0
          continue
        }
        if (in_str) {
          if (ch == "\\") esc = 1
          else if (ch == "\"") in_str = 0
          continue
        }
        if (in_chr) {
          if (ch == "\\") esc = 1
          else if (ch == "\047") in_chr = 0
          continue
        }
        if (ch == "\"") {
          in_str = 1
          continue
        }
        if (ch == "\047") {
          in_chr = 1
          continue
        }
        if (ch == "{") c++
        else if (ch == "}") c--
      }
      return c
    }

    {
      line = $0

      if (is_effective(line)) {
        file_effective++
      }

      if (!in_fn && line ~ /(^|[^[:alnum:]_])fn[[:space:]]+[[:alpha:]_][[:alnum:]_]*/) {
        fn_name = line
        sub(/.*(^|[^[:alnum:]_])fn[[:space:]]+/, "", fn_name)
        sub(/[^[:alnum:]_].*$/, "", fn_name)
        pending_fn = 1
        pending_name = fn_name
        pending_start = NR
      }

      if (pending_fn && index(line, "{") > 0) {
        in_fn = 1
        fn_name = pending_name
        fn_start = pending_start
        fn_effective = 0
        brace_depth = 0
        pending_fn = 0
      }

      if (in_fn) {
        if (is_effective(line)) {
          fn_effective++
        }

        brace_depth += count_braces(line)

        if (brace_depth == 0) {
          if (fn_effective > max_fn_lines) {
            printf("ERROR: function too large in %s: %s (line %d) has %d effective lines (max %d)\n", file, fn_name, fn_start, fn_effective, max_fn_lines)
            has_violation = 1
          }
          in_fn = 0
          fn_name = ""
          fn_start = 0
          fn_effective = 0
        }
      }
    }

    END {
      if (file_effective > max_file_lines) {
        printf("ERROR: file too large: %s has %d effective lines (max %d)\n", file, file_effective, max_file_lines)
        has_violation = 1
      }
      exit has_violation ? 1 : 0
    }
  ' "$file"; then
    violations=1
  fi
done < <(find crates -type f -name '*.rs' ! -path '*/target/*' ! -name '*.pb.rs' -print0)

if [[ "$violations" -ne 0 ]]; then
  exit 1
fi

echo "File size and complexity check passed ✅"
