#! /usr/bin/sh
#
#
# Used in GitHub actions to fuzz utils/buffer for 30s, to determine if there are
# any immediate errors. This is only ran for 30s because that fuzzing can take
# awhile, and waiting on GitHub actions isn't optimal
#
# This script does the following:
#   (1) sets the Rust default to nightly
#   (2) navigates to correct directory to run fuzztests
#   (3) runs rust fuzz test on utils/buffer for 30 seconds
#   (4) if fuzz test fails before 30s runtime, the script exits indicating Failure.
#       Otherwise indicates success.
#
# This script is called by `.github/workflows/fuzz-utils-buffer.yaml` on every PR onto the main branch.
#

echo "Fuzztest script is starting"
# rustup override set nightly
cd fuzz
curl https://sh.rustup.rs -sSf | sh
cargo install cargo-fuzz
echo "Fuzzing begins"
cargo +nightly fuzz run faster -- -rss_limit_mb=5000000000
# if ! timeout 30s cargo +nightly fuzz run faster -- -rss_limit_mb=5000000000
# then
    # echo "Success. Fuzz tests pass"
    # sleep 3s
# else
    # sleep 3s
    # exit -1
# fi
