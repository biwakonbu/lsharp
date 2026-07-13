#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "ERROR: $*" >&2
  exit 1
}

require_linux_x86_64() {
  if [[ "${LSHARP_NATIVE_STAGE0_TRANSPORT_TEST_ALLOW_UNSUPPORTED_HOST:-}" == "1" ]]; then
    return
  fi

  local system_name
  local machine_name
  system_name="$(uname -s)"
  machine_name="$(uname -m)"
  if [[ "$system_name" != "Linux" || "$machine_name" != "x86_64" ]]; then
    die "native stage0 transport requires Linux x86_64 (detected: $system_name $machine_name)"
  fi
}

if [[ $# -ne 4 ]]; then
  die "usage: $0 <compiler.native> <copied-source-root> <relative-entry> <transport-output>"
fi

require_linux_x86_64

compiler_input="$1"
source_root_input="$2"
relative_entry="$3"
transport_output_input="$4"

[[ -d "$source_root_input" ]] || die "source root is not a directory: $source_root_input"
[[ -n "$relative_entry" ]] || die "entry path must stay within source root: $relative_entry"
case "$relative_entry" in
  /*|..|../*|*/../*|*/..)
    die "entry path must stay within source root: $relative_entry"
    ;;
esac

compiler_dir="$(dirname -- "$compiler_input")"
compiler_name="$(basename -- "$compiler_input")"
compiler_dir="$(cd -- "$compiler_dir" && pwd -P)" || die "could not resolve compiler directory: $compiler_input"
compiler="$compiler_dir/$compiler_name"
[[ -x "$compiler" ]] || die "compiler is not executable: $compiler_input"

source_root="$(cd -- "$source_root_input" && pwd -P)" || die "could not resolve source root: $source_root_input"
entry_path="$(readlink -f -- "$source_root/$relative_entry")" || die "could not resolve entry path: $relative_entry"
[[ -f "$entry_path" ]] || die "entry file is not a regular file: $relative_entry"
if [[ "$source_root" != "/" && "$entry_path" != "$source_root"/* ]]; then
  die "entry path must stay within source root: $relative_entry"
fi

source_line_total="$(LC_ALL=C awk 'END { print NR + 0 }' "$entry_path")" || die "could not count source lines: $relative_entry"
[[ "$source_line_total" =~ ^[0-9]+$ ]] || die "could not count source lines: $relative_entry"
(( source_line_total > 0 )) || die "source entry is empty: $relative_entry"

output_dir_input="$(dirname -- "$transport_output_input")"
output_name="$(basename -- "$transport_output_input")"
[[ "$output_name" != "." && "$output_name" != ".." ]] || die "transport output must be a file path: $transport_output_input"
[[ -d "$output_dir_input" ]] || die "transport output directory is not available: $output_dir_input"
output_dir="$(cd -- "$output_dir_input" && pwd -P)" || die "could not resolve transport output directory: $output_dir_input"
transport_output="$output_dir/$output_name"
[[ ! -d "$transport_output" ]] || die "transport output is a directory: $transport_output_input"
[[ "$transport_output" != "$entry_path" ]] || die "transport output must not overwrite the source entry: $transport_output_input"

chunk_size=64
chunk_override="${NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE:-}"
if [[ "$chunk_override" =~ ^[0-9]+$ ]] \
  && [[ ${#chunk_override} -le 9 ]] \
  && (( 10#$chunk_override > 0 )); then
  chunk_size=$((10#$chunk_override))
fi

compiler_timeout=900
timeout_override="${NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS:-}"
if [[ "$timeout_override" =~ ^[0-9]+$ ]] \
  && [[ ${#timeout_override} -le 9 ]] \
  && (( 10#$timeout_override > 0 )); then
  compiler_timeout=$((10#$timeout_override))
fi
command -v timeout >/dev/null 2>&1 || die "timeout command is required for native compiler execution"

work_dir="$(mktemp -d "$output_dir/.native-stage0-transport.XXXXXX")" || die "could not create transport work directory"
transport_tmp=""

cleanup() {
  local status=$?
  rm -rf -- "$work_dir" || true
  if [[ -n "$transport_tmp" ]]; then
    rm -f -- "$transport_tmp" || true
  fi
  trap - EXIT
  exit "$status"
}
trap cleanup EXIT

transport_tmp="$(mktemp "$output_dir/.native-stage0-transport-output.XXXXXX")" || die "could not create transport output"

run_compiler_chunk() {
  local start="$1"
  local end="$2"
  local include_header="$3"
  local include_tail="$4"
  local chunk_stdout
  local chunk_stderr
  local status=0

  chunk_stdout="$(mktemp "$work_dir/chunk.stdout.XXXXXX")"
  chunk_stderr="$(mktemp "$work_dir/chunk.stderr.XXXXXX")"
  if (cd "$source_root" && timeout "$compiler_timeout" "$compiler" "$relative_entry" "$start" "$end" "$include_header" "$include_tail") >"$chunk_stdout" 2>"$chunk_stderr"; then
    :
  else
    status=$?
    echo "ERROR: native compiler failed for range $start-$end (header=$include_header tail=$include_tail exit=$status)" >&2
    if [[ -s "$chunk_stderr" ]]; then
      cat "$chunk_stderr" >&2
    fi
    return "$status"
  fi
  if [[ ! -s "$chunk_stdout" ]]; then
    echo "ERROR: native compiler produced empty output for range $start-$end (header=$include_header tail=$include_tail)" >&2
    if [[ -s "$chunk_stderr" ]]; then
      cat "$chunk_stderr" >&2
    fi
    return 1
  fi
  printf '%s\n' "$chunk_stdout"
}

require_tail() {
  local chunk_output="$1"

  grep -Fx '9000000003' "$chunk_output" >/dev/null \
    || die "native compiler output is missing the transport tail"
}

first_chunk_output="$(run_compiler_chunk 0 "$chunk_size" 1 0)"
header_marker="$(awk 'NR == 1 { print; exit }' "$first_chunk_output")"
function_start_len="$(awk 'NR == 2 { print; exit }' "$first_chunk_output")"
[[ "$header_marker" == "9000000005" ]] || die "native compiler output is missing the transport header"
[[ "$function_start_len" =~ ^[1-9][0-9]*$ ]] || die "native compiler output has an invalid function_start_len: ${function_start_len:-missing}"
cat "$first_chunk_output" >>"$transport_tmp"

chunk_start="$chunk_size"
if (( function_start_len <= chunk_size )); then
  tail_chunk_output="$(run_compiler_chunk "$function_start_len" "$function_start_len" 0 1)"
  require_tail "$tail_chunk_output"
  cat "$tail_chunk_output" >>"$transport_tmp"
else
  while (( chunk_start < function_start_len )); do
    chunk_end=$((chunk_start + chunk_size))
    include_tail=0
    if (( chunk_end >= function_start_len )); then
      chunk_end="$function_start_len"
      include_tail=1
    fi
    chunk_output="$(run_compiler_chunk "$chunk_start" "$chunk_end" 0 "$include_tail")"
    if (( include_tail == 1 )); then
      require_tail "$chunk_output"
    fi
    cat "$chunk_output" >>"$transport_tmp"
    chunk_start="$chunk_end"
  done
fi

[[ -s "$transport_tmp" ]] || die "native compiler produced empty transport output"
mv -f "$transport_tmp" "$transport_output"
transport_tmp=""
