#!/usr/bin/env bash

set -euo pipefail

project_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/AdamantiumLinuxCliProject"

if [[ -z "$project_root" || "$project_root" == "/" ]]; then
    echo "Refusing to use an unsafe test directory." >&2
    exit 1
fi

rm -rf "$project_root"

adamantium --version
help_output="$(adamantium --help)"
grep -Fq "adamantium build [PROJECT_DIRECTORY]" <<< "$help_output"
grep -Fq "adamantium run [PROJECT_DIRECTORY]" <<< "$help_output"
grep -Fq "adamantium test run" <<< "$help_output"

adamantium new "$project_root"
test -f "$project_root/project.toml"
test -f "$project_root/requirement.toml"
test -f "$project_root/code/main.ad"

cat > "$project_root/code/main.ad" <<'ADAMANTIUM'
fun main() {
    print.newline("Linux CLI works");
}
ADAMANTIUM

cat > "$project_root/code/tests.ad" <<'ADAMANTIUM'
#[test]
fun linux_cli_test() {
    print.newline("Linux test works");
}
ADAMANTIUM

adamantium check "$project_root"
adamantium build "$project_root"

executable="$project_root/target/AdamantiumLinuxCliProject"
test -x "$executable"
file "$executable" | grep -Eq "ELF 64-bit.*x86-64"
"$executable" | tr -d '\r' | grep -Fxq "Linux CLI works"
adamantium run "$project_root" | tr -d '\r' | grep -Fxq "Linux CLI works"

test_list="$(adamantium test list "$project_root")"
grep -Fxq "linux_cli_test" <<< "$test_list"
adamantium test run "$project_root" linux_cli_test
adamantium test run "$project_root"

adamantium clean "$project_root"
test ! -e "$project_root/target"
adamantium build "$project_root"
adamantium clear "$project_root"
test ! -e "$project_root/target"
