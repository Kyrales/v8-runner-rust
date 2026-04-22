#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="${V8TR_CI_PLATFORM_WORK_DIR:-${RUNNER_TEMP:-$ROOT_DIR/target/ci-temp}/v8tr-platform}"
EXTRACT_ROOT="$WORK_DIR/extracted"
BUNDLE_URL="${V8TR_PLATFORM_BUNDLE_URL:-}"
BUNDLE_SHA256="${V8TR_PLATFORM_BUNDLE_SHA256:-}"

die() {
    echo "$*" >&2
    exit 2
}

write_env() {
    local key="$1"
    local value="$2"
    local delimiter="__V8TR_ENV__"

    if [[ -n "${GITHUB_ENV:-}" ]]; then
        {
            printf '%s<<%s\n' "$key" "$delimiter"
            printf '%s\n' "$value"
            printf '%s\n' "$delimiter"
        } >> "$GITHUB_ENV"
    else
        printf '%s=%s\n' "$key" "$value"
    fi
}

require_command() {
    local command_name="$1"
    command -v "$command_name" >/dev/null 2>&1 || die "Required command is missing: $command_name"
}

download_bundle() {
    local destination="$1"

    require_command curl
    mkdir -p "$(dirname "$destination")"
    curl --fail --silent --show-error --location --retry 3 --retry-all-errors "$BUNDLE_URL" -o "$destination"
}

verify_checksum() {
    local archive="$1"
    local expected="$2"
    local actual=""

    if [[ -z "$expected" ]]; then
        return 0
    fi

    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$archive" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
    else
        require_command python3
        actual="$(python3 - "$archive" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
print(hashlib.sha256(path.read_bytes()).hexdigest())
PY
)"
    fi

    if [[ "$actual" != "$expected" ]]; then
        die "Platform bundle checksum mismatch: expected $expected, got $actual"
    fi
}

extract_bundle() {
    local archive="$1"
    local destination="$2"

    require_command python3
    python3 - "$archive" "$destination" <<'PY'
import pathlib
import shutil
import stat
import sys
import tarfile
import zipfile

archive = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])

if destination.exists():
    shutil.rmtree(destination)
destination.mkdir(parents=True, exist_ok=True)

archive_name = archive.name.lower()


def normalize_parts(name: str) -> tuple[str, ...]:
    pure = pathlib.PurePosixPath(name.replace("\\", "/"))
    parts = tuple(part for part in pure.parts if part not in ("", "."))
    if pure.is_absolute() or any(part == ".." for part in parts):
        raise SystemExit(f"unsafe path in platform bundle entry: {name}")
    if parts and ":" in parts[0]:
        raise SystemExit(f"unsafe drive-qualified path in platform bundle entry: {name}")
    return parts


def resolve_target(name: str) -> pathlib.Path:
    parts = normalize_parts(name)
    return destination.joinpath(*parts)


if archive_name.endswith(".zip"):
    with zipfile.ZipFile(archive) as zf:
        for member in zf.infolist():
            mode = (member.external_attr >> 16) & 0o170000
            if stat.S_ISLNK(mode):
                raise SystemExit(f"platform bundle must not contain symlinks: {member.filename}")

            target = resolve_target(member.filename)
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue

            target.parent.mkdir(parents=True, exist_ok=True)
            with zf.open(member, "r") as source, open(target, "wb") as fh:
                shutil.copyfileobj(source, fh)

            file_mode = (member.external_attr >> 16) & 0o777
            if file_mode:
                target.chmod(file_mode)
elif archive_name.endswith(".tar.gz") or archive_name.endswith(".tgz") or archive_name.endswith(".tar.xz") or archive_name.endswith(".txz") or archive_name.endswith(".tar"):
    with tarfile.open(archive) as tf:
        for member in tf.getmembers():
            if member.issym() or member.islnk():
                raise SystemExit(f"platform bundle must not contain symlinks: {member.name}")

            target = resolve_target(member.name)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue

            source = tf.extractfile(member)
            if source is None:
                continue

            target.parent.mkdir(parents=True, exist_ok=True)
            with source, open(target, "wb") as fh:
                shutil.copyfileobj(source, fh)

            if member.mode:
                target.chmod(member.mode & 0o777)
else:
    raise SystemExit(f"unsupported platform bundle format: {archive.name}")
PY
}

locate_platform_paths() {
    require_command python3
    python3 - "$EXTRACT_ROOT" <<'PY'
import os
import pathlib
import platform
import sys

root = pathlib.Path(sys.argv[1])
is_windows = platform.system().lower().startswith("win")
binary_name = "1cv8.exe" if is_windows else "1cv8"
ibsrv_name = "ibsrv.exe" if is_windows else "ibsrv"


def pick(name: str) -> pathlib.Path:
    candidates = sorted(
        (
            path
            for path in root.rglob(name)
            if path.is_file()
        ),
        key=lambda path: (len(path.parts), str(path)),
    )
    if not candidates:
        raise SystemExit(f"platform bundle does not contain required utility: {name}")
    return candidates[0]


onecv8 = pick(binary_name)
ibsrv = pick(ibsrv_name)

if onecv8.parent == ibsrv.parent:
    hint = onecv8.parent
else:
    hint = pathlib.Path(os.path.commonpath([str(onecv8.parent), str(ibsrv.parent)]))

print("\t".join([hint.as_posix(), onecv8.as_posix(), ibsrv.as_posix()]))
PY
}

main() {
    [[ -n "$BUNDLE_URL" ]] || die "V8TR_PLATFORM_BUNDLE_URL must be set for platform installation"
    [[ -n "$BUNDLE_SHA256" ]] || die "V8TR_PLATFORM_BUNDLE_SHA256 must be set for trusted platform installation"

    mkdir -p "$WORK_DIR"
    local archive_name="${BUNDLE_URL##*/}"
    archive_name="${archive_name%%\?*}"
    if [[ -z "$archive_name" || "$archive_name" == "$BUNDLE_URL" ]]; then
        archive_name="platform-bundle.tar.gz"
    fi
    local archive_path="$WORK_DIR/$archive_name"
    download_bundle "$archive_path"
    verify_checksum "$archive_path" "$BUNDLE_SHA256"
    extract_bundle "$archive_path" "$EXTRACT_ROOT"

    local platform_hint=""
    local onecv8_path=""
    local ibsrv_path=""
    IFS=$'\t' read -r platform_hint onecv8_path ibsrv_path < <(locate_platform_paths)

    [[ -n "$platform_hint" ]] || die "Failed to resolve platform hint from extracted bundle"
    [[ -n "$onecv8_path" ]] || die "Failed to resolve 1cv8 binary from extracted bundle"
    [[ -n "$ibsrv_path" ]] || die "Failed to resolve ibsrv binary from extracted bundle"

    write_env V8TR_PLATFORM_PATH "$platform_hint"
    write_env V8TR_1CV8_PATH "$onecv8_path"
    write_env V8TR_IBSRV_PATH "$ibsrv_path"
    write_env V8TR_PLATFORM_INSTALL_ROOT "$EXTRACT_ROOT"

    echo "Installed 1C platform bundle into: $EXTRACT_ROOT"
    echo "Resolved platform hint: $platform_hint"
    echo "Resolved ibsrv path: $ibsrv_path"
}

main "$@"
