# Load scripts/field/workloads/complex_apps/catalog.tsv into parallel arrays.
# Requires ROOT to be set. Call: catalog_load  then  catalog_lookup ID

catalog_tsv() {
  printf '%s\n' "${ROOT}/scripts/field/workloads/complex_apps/catalog.tsv"
}

catalog_load() {
  local f line
  f="$(catalog_tsv)"
  CATALOG_IDS=()
  unset CATALOG_TIER CATALOG_FAMILY CATALOG_TOKEN CATALOG_CPAN CATALOG_YUM CATALOG_DRIVER CATALOG_REASON
  declare -gA CATALOG_TIER CATALOG_FAMILY CATALOG_TOKEN CATALOG_CPAN CATALOG_YUM CATALOG_DRIVER CATALOG_REASON
  [[ -f "$f" ]] || {
    printf 'ERROR: missing catalog %s\n' "$f" >&2
    return 1
  }
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    local id tier family token cpan yum driver reason
    IFS=$'\t' read -r id tier family token cpan yum driver reason <<<"$line"
    [[ -n "$id" ]] || continue
    CATALOG_IDS+=("$id")
    CATALOG_TIER[$id]="$tier"
    CATALOG_FAMILY[$id]="$family"
    CATALOG_TOKEN[$id]="$token"
    CATALOG_CPAN[$id]="$cpan"
    CATALOG_YUM[$id]="$yum"
    CATALOG_DRIVER[$id]="$driver"
    CATALOG_REASON[$id]="$reason"
  done <"$f"
}

catalog_lookup() {
  local id="$1"
  [[ -n "${CATALOG_TIER[$id]:-}" ]] || {
    printf 'ERROR: unknown catalog id %s\n' "$id" >&2
    return 1
  }
  APP_ID="$id"
  APP_TIER="${CATALOG_TIER[$id]}"
  APP_FAMILY="${CATALOG_FAMILY[$id]}"
  APP_TOKEN="${CATALOG_TOKEN[$id]}"
  APP_CPAN="${CATALOG_CPAN[$id]}"
  APP_YUM="${CATALOG_YUM[$id]}"
  [[ "$APP_YUM" == "-" ]] && APP_YUM=""
  APP_DRIVER="${CATALOG_DRIVER[$id]}"
  APP_REASON="${CATALOG_REASON[$id]}"
}

catalog_top10_ids() {
  local id
  for id in "${CATALOG_IDS[@]}"; do
    [[ "${CATALOG_TIER[$id]}" == "top10" ]] && printf '%s\n' "$id"
  done
}
