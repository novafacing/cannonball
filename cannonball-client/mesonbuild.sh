#!/bin/bash

# THis file basically just invokes cargo and copies files. Once `--out-dir` is
# stabilized in cargo, this file can be removed and that can bse used instead,
# but I'm not willing to require unstable just for that (see @domenuk on twitter)

set -e

BUILDDIR="${1}"
LIBNAME="${2}"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

if [ -z "${BUILDDIR}" -o -z "${LIBNAME}" ]; then
    echo "Usage: ${0} <build dir> <libname>"
    exit 1
fi

# Make sure the build dir exists
if [ ! -d "${BUILDDIR}" ]; then
    echo "Build dir ${BUILDDIR} does not exist"
    exit 1
fi

cargo build --release --lib --target-dir "${BUILDDIR}" \
    --manifest-path "${SCRIPT_DIR}/Cargo.toml"

cp "${BUILDDIR}/release/${LIBNAME}" "${BUILDDIR}/${LIBNAME}"