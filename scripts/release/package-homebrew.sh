#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"

version=""
url=""
artifact_dir=""
formula_output=""

usage() {
  cat <<'USAGE'
Usage: scripts/release/package-homebrew.sh [options]

Build the eternalMac release binary, package it as a Homebrew tarball, and
render a formula from packaging/homebrew/eternalmac.rb.tmpl.

Options:
  --version <version>          Version to use in the artifact filename.
                               Defaults to Cargo.toml package version.
  --url <url>                  URL to stamp into the formula.
                               Defaults to file://<generated tarball>.
  --artifact-dir <path>        Output directory for generated artifacts.
                               Defaults to target/homebrew.
  --formula-output <path>      Formula output path.
                               Defaults to <artifact-dir>/Formula/eternalmac.rb.
  -h, --help                   Show this help.
USAGE
}

while (($#)); do
  case "$1" in
    --version)
      version="${2:?missing value for --version}"
      shift 2
      ;;
    --url)
      url="${2:?missing value for --url}"
      shift 2
      ;;
    --artifact-dir)
      artifact_dir="${2:?missing value for --artifact-dir}"
      shift 2
      ;;
    --formula-output)
      formula_output="${2:?missing value for --formula-output}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$repo_root"

if [[ -z "$version" ]]; then
  version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
fi

if [[ -z "$version" ]]; then
  echo "could not determine version" >&2
  exit 1
fi

host_triple="$(rustc -vV | awk '/^host:/ { host = $2 } END { print host }')"
if [[ -z "$host_triple" ]]; then
  echo "could not determine Rust host triple" >&2
  exit 1
fi

case "$host_triple" in
  aarch64-apple-darwin)
    arch_requirement="depends_on arch: :arm64"
    ;;
  x86_64-apple-darwin)
    arch_requirement="depends_on arch: :x86_64"
    ;;
  *)
    arch_requirement=""
    ;;
esac

artifact_dir="${artifact_dir:-${repo_root}/target/homebrew}"
formula_output="${formula_output:-${artifact_dir}/Formula/eternalmac.rb}"
package_name="eternalmac-${version}-${host_triple}.tar.gz"
artifact_path="${artifact_dir}/${package_name}"

mkdir -p "$artifact_dir" "$(dirname -- "$formula_output")"

cargo build --release --locked --bin eternalMac

staging_dir="$(mktemp -d "${TMPDIR:-/tmp}/eternalmac-homebrew.XXXXXX")"
trap 'rm -rf "$staging_dir"' EXIT

install -m 0755 "${repo_root}/target/release/eternalMac" "${staging_dir}/eternalMac"
install -m 0644 "${repo_root}/LICENSE" "${staging_dir}/LICENSE"
install -m 0644 "${repo_root}/NOTICE" "${staging_dir}/NOTICE"
tar -C "$staging_dir" -czf "$artifact_path" eternalMac LICENSE NOTICE

sha256="$(shasum -a 256 "$artifact_path" | awk '{ print $1 }')"
if [[ -z "$url" ]]; then
  url="$(ruby -ruri -e 'puts "file://" + URI::DEFAULT_PARSER.escape(File.expand_path(ARGV.fetch(0)))' "$artifact_path")"
fi

VERSION="$version" \
URL="$url" \
SHA256="$sha256" \
ARCH_REQUIREMENT="$arch_requirement" \
  perl -pe '
    s/\{\{\s*\.Version\s*\}\}/$ENV{VERSION}/g;
    s/\{\{\s*\.URL\s*\}\}/$ENV{URL}/g;
    s/\{\{\s*\.SHA256\s*\}\}/$ENV{SHA256}/g;
    s/\{\{\s*\.ArchRequirement\s*\}\}/$ENV{ARCH_REQUIREMENT}/g;
  ' packaging/homebrew/eternalmac.rb.tmpl > "$formula_output"

cat <<OUTPUT
artifact: $artifact_path
sha256:   $sha256
formula:  $formula_output
url:      $url

Local test:
  scripts/release/install-homebrew-local.sh --formula "$formula_output"
OUTPUT
