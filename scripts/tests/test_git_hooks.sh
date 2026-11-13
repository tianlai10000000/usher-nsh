#!/usr/bin/env bash
# Tests the pre-commit checks (lefthook.yml and scripts/git-hooks/) in a throwaway repository,
# by committing real files and checking whether the commit was allowed and what it contained.
#
# These replace the cases of the earlier hand-written hook, plus the ones lefthook adds:
# the formatter now FIXES files, so what lands in the commit and what is left in the working
# copy both have to be right, and a partly staged file must not drag unstaged work along.
#
# Needs: lefthook, rustfmt, git. shellcheck and actionlint cases are skipped when absent.
set -uo pipefail

source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
command -v lefthook > /dev/null 2>&1 || { echo "lefthook is not installed (brew install lefthook)" >&2; exit 2; }
command -v rustfmt > /dev/null 2>&1 || { echo "rustfmt is not installed" >&2; exit 2; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work" || exit 1
git init -q -b main .
git config user.email test@example.com
git config user.name test
mkdir -p scripts/git-hooks scripts/ci .github/workflows src
cp "$source_root/lefthook.yml" .
cp "$source_root"/scripts/git-hooks/*.sh scripts/git-hooks/
printf '[package]\nname = "t"\nedition = "2024"\n' > Cargo.toml
# The scaffolding is committed once with hooks off, so a case's cleanup removes only its own files.
git add -A
git -c core.hooksPath=/dev/null commit -qm scaffolding
lefthook install > /dev/null 2>&1

failures=0
pass() { echo "ok    $1"; }
# verdict: report $2 if the command succeeded, otherwise $3. Written out as an if, so a
# failure of pass itself can never be mistaken for a failed check.
verdict() {
    local status=$1
    if [[ "$status" -eq 0 ]]; then pass "$2"; else fail "$3"; fi
}
fail() {
    echo "FAIL  $1"
    [[ -f "$work/out" ]] && sed 's/\x1b\[[0-9;]*m//g' "$work/out" | head -8 | sed 's/^/        | /'
    failures=$((failures + 1))
}
reset() {
    git reset -q --hard 2> /dev/null || true
    git clean -qf
    mkdir -p src scripts/ci .github/workflows
}
# commit_is WANT  : the commit's exit status must be WANT (0 allowed, non-zero refused)
commit() { git commit -qm test > "$work/out" 2>&1; }

# 1. unformatted Rust is FIXED, and the commit holds the fixed content
printf 'fn main(){let x=1;}\n' > src/a.rs; git add src/a.rs
if commit && [[ "$(git show HEAD:src/a.rs)" == $'fn main() {\n    let x = 1;\n}' ]]; then pass "unformatted Rust is formatted and the commit holds the formatted file"; else fail "unformatted Rust was not fixed into the commit"; fi
ok=1
if [[ "$(cat src/a.rs)" == "$(git show HEAD:src/a.rs)" ]]; then ok=0; fi
verdict "$ok" "  the working copy matches what was committed" "  the working copy differs from the commit"
reset

# 2. already formatted: allowed
printf 'fn main() {\n    let x = 1;\n}\n' > src/b.rs; git add src/b.rs
commit
verdict $? "formatted Rust is allowed" "formatted Rust was refused"
reset

# 3. partial staging: unstaged work must stay out of the commit and stay on disk
printf 'fn one() {}\n' > src/p.rs; git add src/p.rs; git -c core.hooksPath=/dev/null commit -qm base
printf 'fn one(){let a=1;}\n' | git hash-object -w --stdin > "$work/blob"
git update-index --cacheinfo 100644,"$(cat "$work/blob")",src/p.rs
printf 'fn one(){let a=1;}\n\nfn UNSTAGED_WORK(){}\n' > src/p.rs
if commit; then
    committed="$(git show HEAD:src/p.rs)"
    ok=1
    if [[ "$committed" != *UNSTAGED_WORK* ]]; then ok=0; fi
    verdict "$ok" "partial staging: unstaged work stays out of the commit" "partial staging: unstaged work was swept into the commit"
    ok=1
    if [[ "$committed" == *$'fn one() {\n    let a = 1;\n}'* ]]; then ok=0; fi
    verdict "$ok" "  and the staged part was formatted" "  the staged part was not formatted"
    grep -q UNSTAGED_WORK src/p.rs
    verdict $? "  and the unstaged work is still in the working copy" "  the unstaged work was LOST from the working copy"
else
    fail "partial staging: the commit was refused"
fi
reset

# 4. the mod trap: formatting one staged file must not rewrite a file next to it that was not staged
printf 'mod other;\nfn main() {}\n' > src/lib.rs; printf 'fn x() {}\n' > src/other.rs
git add src/lib.rs src/other.rs; git -c core.hooksPath=/dev/null commit -qm base
printf 'mod other;\nfn main(){}\n' > src/lib.rs; git add src/lib.rs
printf 'fn x(){ let  typed_by_hand=1; }\n' > src/other.rs
commit
if [[ "$(cat src/other.rs)" == 'fn x(){ let  typed_by_hand=1; }' ]]; then pass "a file that was not staged is not rewritten (rustfmt does not follow mod)"; else fail "a file that was NOT staged was rewritten by the formatter"; fi
reset

# 5. a syntax error stops the commit and leaves the file alone
printf 'fn main( {\n' > src/c.rs; git add src/c.rs
if commit; then fail "a Rust syntax error was committed"; else pass "a Rust syntax error stops the commit"; fi
ok=1
if [[ "$(cat src/c.rs)" == 'fn main( {' ]]; then ok=0; fi
verdict "$ok" "  and the file is left exactly as it was" "  the file was changed or truncated"
reset

# 6. file names with spaces
printf 'fn main(){}\n' > "src/my file.rs"; git add "src/my file.rs"
if commit && [[ "$(git show 'HEAD:src/my file.rs')" == 'fn main() {}' ]]; then pass "a file name with a space is formatted and committed"; else fail "a file name with a space was not handled"; fi
reset

# 7. a formatted file that declares a mod which is not staged
printf 'fn main() {}\n\nmod elsewhere;\n' > src/m.rs; git add src/m.rs
commit
verdict $? "a formatted file declaring a mod that does not exist is allowed" "a file declaring a missing mod was refused"
reset

# 8. key files
echo x > key.secret; git add -f key.secret
if commit; then fail "a staged *.secret was committed"; else pass "a staged *.secret is refused"; fi
reset
mkdir -p keys/deep; echo x > keys/deep/id.pem; git add -f keys/deep/id.pem
if commit; then fail "a staged .pem in a subdirectory was committed"; else pass "a staged .pem in a subdirectory is refused"; fi
reset

# 9. a private key block pasted into another file
printf '%s\n' '-----BEGIN PRIVATE KEY-----' 'abc' > notes.txt; git add notes.txt
if commit; then fail "a pasted private key block was committed"; else pass "a pasted private key block is refused"; fi
reset

# 10. workflows
if command -v actionlint > /dev/null 2>&1; then
    printf 'name: x\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    needs: [nope]\n    steps:\n      - run: echo hi\n' > .github/workflows/x.yml
    git add .github/workflows/x.yml
    if commit; then fail "a broken workflow was committed"; else pass "a broken workflow is refused"; fi
    reset
    printf 'name: x\non: push\njobs:\n  a:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: echo hi\n' > .github/workflows/ok.yml
    git add .github/workflows/ok.yml
    commit
    verdict $? "a valid workflow is allowed" "a valid workflow was refused"
    reset

    # Both configuration names must use the index, including when the working copy differs.
    for config in .github/actionlint.yaml .github/actionlint.yml; do
        printf 'name: native\non: push\njobs:\n  a:\n    runs-on: [self-hosted, macOS, test-native]\n    steps:\n      - run: echo hi\n' > .github/workflows/native.yml
        printf 'self-hosted-runner:\n  labels:\n    - test-native\n' > "$config"
        git add .github/workflows/native.yml "$config"
        printf 'self-hosted-runner:\n  labels: []\n' > "$config"
        commit
        verdict $? "a custom runner uses staged $config despite unstaged edits" "staged $config was ignored"
        # Remove the committed fixture so each configuration name is checked independently.
        git rm -qf --ignore-unmatch .github/workflows/native.yml "$config"
        git -c core.hooksPath=/dev/null commit -qm cleanup
        reset

        printf 'name: native\non: push\njobs:\n  a:\n    runs-on: [self-hosted, macOS, test-native]\n    steps:\n      - run: echo hi\n' > .github/workflows/native.yml
        printf 'self-hosted-runner:\n  labels:\n    - test-native\n' > "$config"
        git add .github/workflows/native.yml
        if commit; then fail "unstaged $config allowed an unknown runner"; else pass "unstaged $config cannot allow an unknown runner"; fi
        rm -f "$config"
        reset
    done
else
    echo "skip  workflow cases (actionlint is not installed)"
fi

# 11. shell scripts
if command -v shellcheck > /dev/null 2>&1; then
    # Unquoted on purpose (shellcheck SC2086): written literally so it is not expanded here.
    # shellcheck disable=SC2016
    printf '%s\n' '#!/bin/bash' 'echo $1' > scripts/ci/bad.sh
    git add scripts/ci/bad.sh
    if commit; then fail "a script shellcheck rejects was committed"; else pass "a script shellcheck rejects is refused"; fi
    reset
else
    echo "skip  shellcheck case (shellcheck is not installed)"
fi

# 12. the hook configuration itself: a broken lefthook.yml must be refused, not silently ignored
cp lefthook.yml "$work/lefthook.good"
printf '%s\n' 'pre-commit:' '  jobs:' '    - name: x' '   bad indent: [' > lefthook.yml
git add lefthook.yml
if commit; then fail "a broken lefthook.yml was committed"; else pass "a broken lefthook.yml is refused"; fi
cp "$work/lefthook.good" lefthook.yml
reset

# 12b. the gap lefthook leaves: a config that is ALREADY broken when `lefthook install` runs writes
# no hook at all, so later commits are not checked, and nothing says so. The CI step runs
# `lefthook validate` for exactly this; here we only prove the gap is real, so the CI step is
# known to be necessary and the docs are right to tell people to check the install.
hold="$(mktemp -d)"
git -C "$hold" init -q -b main
printf '%s\n' 'pre-commit:' '  jobs:' '    - name: x' '   bad indent: [' > "$hold/lefthook.yml"
if (cd "$hold" && lefthook install > /dev/null 2>&1); then
    fail "lefthook install accepted a broken config (the documented gap is gone: revisit the docs and CI)"
elif [[ ! -f "$hold/.git/hooks/pre-commit" ]]; then
    pass "a broken config at install time writes no hook (the gap CI's validate step covers)"
else
    fail "unexpected: install failed but left a hook behind"
fi
rm -rf "$hold"

# 13. unrelated files and an empty change
echo hi > README.md; git add README.md
commit
verdict $? "an unrelated file is allowed" "an unrelated file was refused"

# 14. the job order: the formatter runs before the checks that read the tree
order="$(grep -E '^    - name:' lefthook.yml | sed 's/.*name: //' | tr '\n' ' ')"
ok=1
if [[ "$order" == format-rust\ * ]]; then ok=0; fi
verdict "$ok" "the formatter is the first job" "the formatter is not first: $order"

# 15. a clean config validates
lefthook validate > "$work/out" 2>&1
verdict $? "lefthook.yml validates" "lefthook.yml does not validate"

if [[ $failures -ne 0 ]]; then
    echo "$failures hook test(s) failed" >&2
    exit 1
fi
echo "all hook tests passed"
