#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-validate}"
MANIFEST="${2:-${ROOT_DIR}/allocations/production.json}"
EXPECTED_SHA256="2655f63831491234041941f0bd1fae45dbed84e82b189d91f5957efa55317737"
EXPECTED_TOTAL="37442405803143"

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

command -v jq >/dev/null 2>&1 || die "jq is required"
command -v shasum >/dev/null 2>&1 || die "shasum is required"
[[ -f "${MANIFEST}" ]] || die "allocation manifest not found: ${MANIFEST}"

actual_sha256="$(shasum -a 256 "${MANIFEST}")"
actual_sha256="${actual_sha256%% *}"
[[ "${actual_sha256}" == "${EXPECTED_SHA256}" ]] ||
    die "production allocation manifest SHA-256 mismatch"

jq -e --arg total "${EXPECTED_TOTAL}" '
    . == {
        schema_version: 1,
        allocation_total_raw: $total,
        allocations: [
            {
                beneficiary: "GBDVBOWFPYWJS3BVS4H4YYOGN7UTMR4TKKPJ77YK2N7LD7OVYI35NN52",
                amount_raw: "8892037498416"
            },
            {
                beneficiary: "GCQEFBTWQMZMKS2PAFUKQN7QE5YQ4SFSU5PTRAV7WETVT6LNKDE5ARA7",
                amount_raw: "7597459398743"
            },
            {
                beneficiary: "GANNTI6OTKR6WSOVOIGUMT7GMSSEPV3UL3W4VI22VLFW3NT5ABOZ2YFB",
                amount_raw: "7600947904368"
            },
            {
                beneficiary: "GBJXBKZ643REPENN3NBTVGNH3DCDJGVYQE3QDA37WMAERN4JE4HTZQCW",
                amount_raw: "13351961001616"
            }
        ]
    } and
    ([.allocations[].beneficiary] | unique | length) == 4 and
    ([.allocations[].amount_raw | tonumber] | add | tostring) == $total
' "${MANIFEST}" >/dev/null || die "production allocation manifest is not canonical"

case "${MODE}" in
    validate)
        printf 'Validated canonical production allocations: 4 beneficiaries, %s raw BLNT.\n' \
            "${EXPECTED_TOTAL}"
        ;;
    print-constructor-arg)
        jq -c '[.allocations[] | [.beneficiary, .amount_raw]]' "${MANIFEST}"
        ;;
    self-test)
        invalid_manifest="$(mktemp "${TMPDIR:-/tmp}/blnt-lockup-allocations.XXXXXX")"
        trap 'rm -f -- "${invalid_manifest}"' EXIT
        jq '.allocations[0].amount_raw = "1"' "${MANIFEST}" >"${invalid_manifest}"
        if "${BASH_SOURCE[0]}" validate "${invalid_manifest}" >/dev/null 2>&1; then
            die "tampered production allocation manifest unexpectedly passed"
        fi
        printf 'Validated canonical production allocations and rejected tampered input.\n'
        ;;
    *)
        die "usage: $0 [validate|print-constructor-arg|self-test] [manifest.json]"
        ;;
esac
