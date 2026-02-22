#!/usr/bin/env bash
# File size & complexity checker for Swift files.
# Known limitations (acceptable for our codebase):
#   - Nested block comments (/* /* */ */) not tracked (uses boolean, not depth counter)
#   - Multi-line string literals (""" ... """) not tracked across lines
#   - Only standard named functions detected (no backticked/operator funcs)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

max_file_lines=300
max_fn_lines=50
violations=0

while IFS= read -r -d '' file; do
  if ! awk -v file="$file" -v max_file_lines="$max_file_lines" -v max_fn_lines="$max_fn_lines" '
    function is_effective(line) {
      return line !~ /^[[:space:]]*$/
    }

    function strip_comments(line,   out, i, ch, next_ch, in_str, esc) {
      out = ""
      in_str = 0
      esc = 0
      i = 1

      while (i <= length(line)) {
        ch = substr(line, i, 1)
        next_ch = (i < length(line) ? substr(line, i + 1, 1) : "")

        if (in_block_comment) {
          if (ch == "*" && next_ch == "/") {
            in_block_comment = 0
            i += 2
          } else {
            i++
          }
          continue
        }

        if (esc) {
          out = out ch
          esc = 0
          i++
          continue
        }

        if (in_str) {
          out = out ch
          if (ch == "\\") esc = 1
          else if (ch == "\"") in_str = 0
          i++
          continue
        }

        if (ch == "\"") {
          out = out ch
          in_str = 1
          i++
          continue
        }

        if (ch == "/" && next_ch == "/") {
          break
        }

        if (ch == "/" && next_ch == "*") {
          in_block_comment = 1
          i += 2
          continue
        }

        out = out ch
        i++
      }

      return out
    }

    function count_braces(cleaned,   i, ch, in_str, esc, c) {
      c = 0
      in_str = 0
      esc = 0

      for (i = 1; i <= length(cleaned); i++) {
        ch = substr(cleaned, i, 1)

        if (esc) {
          esc = 0
          continue
        }

        if (in_str) {
          if (ch == "\\") esc = 1
          else if (ch == "\"") in_str = 0
          continue
        }

        if (ch == "\"") {
          in_str = 1
          continue
        }

        if (ch == "{") c++
        else if (ch == "}") c--
      }

      return c
    }

    function has_structural_open_brace(cleaned,   i, ch, in_str, esc) {
      in_str = 0
      esc = 0

      for (i = 1; i <= length(cleaned); i++) {
        ch = substr(cleaned, i, 1)

        if (esc) {
          esc = 0
          continue
        }

        if (in_str) {
          if (ch == "\\") esc = 1
          else if (ch == "\"") in_str = 0
          continue
        }

        if (ch == "\"") {
          in_str = 1
          continue
        }

        if (ch == "{") return 1
      }

      return 0
    }

    {
      line = $0
      clean = strip_comments(line)

      if (is_effective(clean)) {
        file_effective++
      }

      # NOTE: This regex does not cover backticked names (`func `default`()`) or operator functions.
      if (!in_fn && !pending_fn && clean ~ /(^|[^[:alnum:]_])func[[:space:]]+[[:alpha:]_][[:alnum:]_]*[[:space:]]*\(/) {
        fn_name = clean
        sub(/.*(^|[^[:alnum:]_])func[[:space:]]+/, "", fn_name)
        sub(/[[:space:]]*\(.*/, "", fn_name)
        pending_fn = 1
        pending_name = fn_name
        pending_start = NR
      }

      if (pending_fn && has_structural_open_brace(clean)) {
        in_fn = 1
        fn_name = pending_name
        fn_start = pending_start
        fn_effective = 0
        brace_depth = 0
        pending_fn = 0
      }

      if (in_fn) {
        if (is_effective(clean)) {
          fn_effective++
        }

        brace_depth += count_braces(clean)

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
done < <(find AgentDemo -type f -name '*.swift' ! -path '*/.build/*' -print0)

if [[ "$violations" -ne 0 ]]; then
  exit 1
fi

echo "File size and complexity check passed ✅"
